#pragma once

// FFI ke librplkit_runtime.a (Rust) — satu-satunya jalan C++ ke runner.
// Kontrak string: pengembalian milik pemanggil, bebaskan via rplkit_str_free.

#ifdef __cplusplus
extern "C" {
#endif

// JSON array [{"name","runtime","description","entry"}] (wajib free).
// "runtime" = native | python | typescript; entry absolut (kecuali native).
char* rplkit_list_tools(const char* repo_root);

// Eksekusi via chain. args_json = JSON array string.
// Return {"code":n,"executed_by":"..."} (wajib free). Output tool
// passthrough ke stdout, sama seperti biner Rust.
char* rplkit_run_tool(const char* repo_root, const char* tool_name, const char* args_json);

void rplkit_str_free(char* s);

// Versi runtime (pointer statis — JANGAN di-free).
const char* rplkit_version();

#ifdef __cplusplus
}  // extern "C"
#endif
