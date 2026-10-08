import { PhaseMirrorAgent } from './index.js';
import * as fs from 'fs';
import * as path from 'path';

async function main() {
  const agent = new PhaseMirrorAgent();
  await agent.load();
  
  const systemPromptPath = path.join(process.cwd(), '../phase-mirror-gpt/prompts/system_prompt.txt');
  let promptText = "The Phase of Mirror Dissonance... (fallback)";
  try {
    promptText = fs.readFileSync(systemPromptPath, 'utf8');
  } catch (e) {}

  const query = promptText + "\n\n---\nInput:\n\nDraft Hash: 123\nPlan: Hello";
  
  const result = await agent.analyze(query, 'You are the Phase Mirror Agent, an AI cognitive assistant. Analyze the user request and provide deterministic feedback on potential drift.');
  console.log("RESULT length:", result.length);
  console.log("RESULT start:", result.substring(0, 100));
}

main();
