#!/usr/bin/env bash
set -euo pipefail

# ── Configuration ──────────────────────────────────────────
CLUSTER_NAME="phase-mirror-test"
NAMESPACE="phase-mirror"
DOMAIN="agent.local"
HELM_CHART="./deploy/helm/phase-mirror-agent"
IMAGE_TAG="local"
DELETE_CLUSTER=false
AUTH_HEADER=""

if [[ "${1:-}" == "--delete" ]]; then
  DELETE_CLUSTER=true
fi

# ── Prerequisites check ────────────────────────────────────
command -v docker   >/dev/null 2>&1 || { echo "docker required"; exit 1; }
command -v kind     >/dev/null 2>&1 || { echo "kind required"; exit 1; }
command -v kubectl  >/dev/null 2>&1 || { echo "kubectl required"; exit 1; }
command -v helm     >/dev/null 2>&1 || { echo "helm required"; exit 1; }
command -v openssl  >/dev/null 2>&1 || { echo "openssl required"; exit 1; }
command -v jq       >/dev/null 2>&1 || { echo "jq required"; exit 1; }

# ── 1. Create kind cluster ─────────────────────────────────
echo "🛠 Creating kind cluster '$CLUSTER_NAME'…"
cat <<EOF | kind create cluster --name "$CLUSTER_NAME" --config=-
kind: Cluster
apiVersion: kind.x-k8s.io/v1alpha4
nodes:
- role: control-plane
  extraPortMappings:
  - containerPort: 80
    hostPort: 80
    protocol: TCP
  - containerPort: 443
    hostPort: 443
    protocol: TCP
EOF

# ── 2. Install ingress-nginx (kind-compatible) ─────────────
echo "📦 Installing ingress-nginx for kind…"
kubectl apply -f https://raw.githubusercontent.com/kubernetes/ingress-nginx/main/deploy/static/provider/kind/deploy.yaml
echo "⏳ Waiting for ingress-nginx to be ready…"
kubectl wait --namespace ingress-nginx \
  --for=condition=ready pod \
  --selector=app.kubernetes.io/component=controller \
  --timeout=120s

# ── 3. Build and load the agent Docker image ───────────────
echo "🐳 Building agent Docker image…"
# Assumes you already ran `cargo build --release --bin phase-mirror-agent`
docker build -t "phase-mirror-agent:$IMAGE_TAG" \
  -f packages/phase-mirror-agent/Dockerfile .
kind load docker-image "phase-mirror-agent:$IMAGE_TAG" --name "$CLUSTER_NAME"

# ── 4. Generate self-signed TLS certificate ────────────────
echo "🔐 Generating self-signed TLS certificate for $DOMAIN…"
openssl req -x509 -nodes -days 1 -newkey rsa:2048 \
  -keyout /tmp/tls.key -out /tmp/tls.crt \
  -subj "/CN=$DOMAIN" -addext "subjectAltName=DNS:$DOMAIN"

kubectl create namespace "$NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -
kubectl create secret tls phase-mirror-agent-tls \
  --cert=/tmp/tls.crt --key=/tmp/tls.key -n "$NAMESPACE" \
  --dry-run=client -o yaml | kubectl apply -f -

# ── 5. Deploy the Helm chart ────────────────────────────────
echo "☸ Deploying Phase Mirror Agent via Helm…"
helm upgrade --install phase-mirror-agent "$HELM_CHART" \
  --namespace "$NAMESPACE" \
  --set image.repository="phase-mirror-agent" \
  --set image.tag="$IMAGE_TAG" \
  --set image.pullPolicy="Never" \
  --set ingress.enabled=true \
  --set 'ingress.hosts[0].host=agent.local' \
  --set 'ingress.hosts[0].paths[0].path=/' \
  --set 'ingress.hosts[0].paths[0].pathType=Prefix' \
  --set 'ingress.hosts[0].paths[0].backendPort=8080' \
  --set 'ingress.tls[0].secretName=phase-mirror-agent-tls' \
  --set 'ingress.tls[0].hosts[0]=agent.local' \
  --set secrets.operatorKey="$(openssl rand -base64 32 | base64 -w0)" \
  --set persistence.enabled=false \
  --wait

echo "⏳ Waiting for agent pod to be ready…"
kubectl wait --namespace "$NAMESPACE" \
  --for=condition=ready pod \
  --selector=app=phase-mirror-agent \
  --timeout=120s

# ── 6. Add local DNS entry (if needed) ──────────────────────
if ! grep -q "$DOMAIN" /etc/hosts; then
  echo "127.0.0.1 $DOMAIN" | sudo tee -a /etc/hosts > /dev/null
fi

# ── 7. Provision operator key for authenticated routes ───────
echo "🔑 Generating operator key for integration tests…"
RAW_KEY=$(bash scripts/gen-operator-key.sh integration-test operator:write | grep 'pmr_op_' | head -1)
KEY_HASH=$(echo "$RAW_KEY" | sha256sum | awk '{print $1}')
kubectl create secret generic phase-mirror-agent-keys \
  --from-literal=operator.key="${KEY_HASH}" \
  --namespace "$NAMESPACE" \
  --dry-run=client -o yaml | kubectl apply -f -
kubectl set env statefulset/phase-mirror-agent -n "$NAMESPACE" \
  PHASE_MIRROR_OPERATOR_KEYS_FILE=/etc/phase-mirror/keys/operator.key
kubectl rollout status statefulset -n "$NAMESPACE" phase-mirror-agent --timeout=120s

# Resolve service ClusterIP for direct testing
SERVICE_IP=$(kubectl get svc -n "$NAMESPACE" phase-mirror-agent -o jsonpath='{.spec.clusterIP}')
AUTH_HEADER="Authorization: Bearer $RAW_KEY"

# ── 8. Canonical test fixtures ──────────────────────────────
echo "🧪 Running canonical test fixtures…"

# Test 1: Valid deploy
echo "  → Test 1: Valid deploy"
RESP1=$(curl -s -k -X POST "http://$SERVICE_IP:8080/api/command" \
  -H "Content-Type: application/json" \
  -H "$AUTH_HEADER" \
  -d '{"text":"deploy web-service on cluster with replicas 3"}')
echo "    Response: $RESP1"
echo "$RESP1" | jq -e '.reply' > /dev/null || { echo "❌ Valid deploy failed"; exit 1; }
echo "    ✅ Passed"

# Test 2: Invalid "all" – should be rejected with 422
echo "  → Test 2: Invalid unbounded quantifier"
RESP2=$(curl -s -k -X POST "http://$SERVICE_IP:8080/api/command" \
  -H "Content-Type: application/json" \
  -H "$AUTH_HEADER" \
  -d '{"text":"deploy all cluster"}')
HTTP_CODE2=$(curl -s -o /dev/null -w "%{http_code}" -k -X POST "http://$SERVICE_IP:8080/api/command" \
  -H "Content-Type: application/json" \
  -H "$AUTH_HEADER" \
  -d '{"text":"deploy all cluster"}')
echo "    HTTP Code: $HTTP_CODE2"
if [[ "$HTTP_CODE2" != "422" ]]; then
  echo "❌ Expected 422 Unprocessable Entity, got $HTTP_CODE2"
  exit 1
fi
echo "    ✅ Passed (command rejected with invariant violation)"

# Test 3: Unknown command – should be rejected with 400
echo "  → Test 3: Unknown command"
RESP3=$(curl -s -k -X POST "http://$SERVICE_IP:8080/api/command" \
  -H "Content-Type: application/json" \
  -H "$AUTH_HEADER" \
  -d '{"text":"revoke it"}')
HTTP_CODE3=$(curl -s -o /dev/null -w "%{http_code}" -k -X POST "http://$SERVICE_IP:8080/api/command" \
  -H "Content-Type: application/json" \
  -H "$AUTH_HEADER" \
  -d '{"text":"revoke it"}')
echo "    HTTP Code: $HTTP_CODE3"
if [[ "$HTTP_CODE3" != "400" ]]; then
  echo "❌ Expected 400 Bad Request, got $HTTP_CODE3"
  exit 1
fi
echo "    ✅ Passed (command rejected as unknown)"

# ── 9. Audit API ────────────────────────────────────────────
echo "📋 Checking audit API…"
AUDIT=$(curl -s -k "http://$SERVICE_IP:8080/audit/entries" \
  -H "$AUTH_HEADER")
echo "    Entries: $AUDIT"
echo "$AUDIT" | jq -e '.entries | length > 0' > /dev/null || { echo "❌ Audit API empty"; exit 1; }
echo "    ✅ Audit API populated"

# ── 10. Metrics endpoint ─────────────────────────────────────
echo "📊 Checking Prometheus metrics…"
METRICS=$(curl -s -k "http://$SERVICE_IP:8080/metrics" -H "$AUTH_HEADER")
echo "$METRICS" | grep -q 'pm_commands_total' && echo "    pm_commands_total found" || { echo "❌ pm_commands_total missing"; exit 1; }
echo "$METRICS" | grep -q 'pm_http_requests_total' && echo "    pm_http_requests_total found" || { echo "❌ pm_http_requests_total missing"; exit 1; }
echo "    ✅ Metrics endpoint valid"

# ── Cleanup (optional) ──────────────────────────────────────
if $DELETE_CLUSTER; then
  echo "🗑 Deleting kind cluster…"
  kind delete cluster --name "$CLUSTER_NAME"
  sudo sed -i "/$DOMAIN/d" /etc/hosts
else
  echo "ℹ️  Cluster '$CLUSTER_NAME' remains running. Use '--delete' to tear down."
fi

echo "🎤 All integration tests passed! The Phase Mirror Agent is production-ready."
