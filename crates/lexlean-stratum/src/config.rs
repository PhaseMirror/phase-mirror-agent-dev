//! Resource and compilation limits (SPEC.md §25.5).

/// The resource limits for compiler and child process execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub max_file_bytes: u64,
    pub max_total_source_bytes: u64,
    pub max_primitive_atoms: u64,
    pub max_token_lattice_edges: u64,
    pub max_parse_states: u64,
    pub max_ir_nodes: u64,
    pub max_scope_depth: u64,
    pub max_import_depth: u64,
    pub max_diagnostics: u64,
    pub max_child_output_bytes: u64,
    pub child_timeout_ms: u64,
}
