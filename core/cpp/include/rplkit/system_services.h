#pragma once

#include <cstdint>
#include <string>
#include <vector>

namespace rplkit {

struct ProcessResult {
    int exit_code = -1;
    std::string output;
};

namespace sys {

// Append a timestamped line to stderr. Never throws.
void log(const std::string& level, const std::string& message);

// Repository helpers.
bool file_exists(const std::string& path);
std::int64_t file_size(const std::string& path);  // -1 when unavailable
std::int64_t count_lines(const std::string& path);  // -1 when unavailable

// Run a shell command, capturing merged stdout+stderr. Never throws.
ProcessResult run_process(const std::string& command);

// Quote one argv element for POSIX sh.
std::string shell_quote(const std::string& arg);

// Minimal JSON helper: extract top-level "key": "value" (string only).
// Returns empty string when absent. Enough for tool manifests and
// config/default.json without pulling in a JSON library.
std::string json_string_field(const std::string& json_text, const std::string& key);

// Read an entire file into a string. Empty optional when unreadable.
std::string read_file(const std::string& path);

}  // namespace sys
}  // namespace rplkit
