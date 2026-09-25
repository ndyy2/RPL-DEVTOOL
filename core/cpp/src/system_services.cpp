#include "rplkit/system_services.h"

#include <array>
#include <chrono>
#include <cstdio>
#include <ctime>
#include <fstream>
#include <iomanip>
#include <iostream>
#include <sstream>

#if defined(_WIN32)
#define RPLKIT_POPEN _popen
#define RPLKIT_PCLOSE _pclose
#else
#define RPLKIT_POPEN popen
#define RPLKIT_PCLOSE pclose
#endif

namespace rplkit {
namespace sys {

void log(const std::string& level, const std::string& message) {
    try {
        auto now = std::chrono::system_clock::now();
        std::time_t t = std::chrono::system_clock::to_time_t(now);
        std::tm tm{};
#if defined(_WIN32)
        gmtime_s(&tm, &t);
#else
        gmtime_r(&t, &tm);
#endif
        std::cerr << "[" << std::put_time(&tm, "%Y-%m-%dT%H:%M:%SZ") << "] [" << level << "] "
                  << message << "\n";
    } catch (...) {
    }
}

bool file_exists(const std::string& path) {
    try {
        std::ifstream f(path, std::ios::binary);
        return static_cast<bool>(f);
    } catch (...) {
        return false;
    }
}

std::int64_t file_size(const std::string& path) {
    try {
        std::ifstream f(path, std::ios::binary | std::ios::ate);
        if (!f) return -1;
        return static_cast<std::int64_t>(f.tellg());
    } catch (...) {
        return -1;
    }
}

std::int64_t count_lines(const std::string& path) {
    try {
        std::ifstream f(path);
        if (!f) return -1;
        std::int64_t n = 0;
        std::string line;
        while (std::getline(f, line)) ++n;
        return n;
    } catch (...) {
        return -1;
    }
}

ProcessResult run_process(const std::string& command) {
    ProcessResult r;
    try {
#if defined(_WIN32)
        std::string wrapped = "cmd /c \"" + command + " 2>&1\"";
#else
        std::string wrapped = command + " 2>&1";
#endif
        std::array<char, 4096> buf{};
        std::string out;
        FILE* pipe = RPLKIT_POPEN(wrapped.c_str(), "r");
        if (!pipe) {
            r.output = "failed to spawn process";
            return r;
        }
        while (std::fgets(buf.data(), static_cast<int>(buf.size()), pipe) != nullptr) {
            out += buf.data();
        }
        int rc = RPLKIT_PCLOSE(pipe);
        r.exit_code = rc;
        r.output = out;
    } catch (...) {
        r.exit_code = -1;
    }
    return r;
}

std::string shell_quote(const std::string& arg) {
    std::string q = "'";
    for (char c : arg) {
        if (c == '\'') {
            q += "'\\''";
        } else {
            q += c;
        }
    }
    q += "'";
    return q;
}

std::string read_file(const std::string& path) {
    try {
        std::ifstream f(path, std::ios::binary);
        if (!f) return {};
        std::ostringstream ss;
        ss << f.rdbuf();
        return ss.str();
    } catch (...) {
        return {};
    }
}

std::string json_string_field(const std::string& json_text, const std::string& key) {
    // Looks for "key" : "value" with optional whitespace. Handles \" escapes.
    std::string needle = "\"" + key + "\"";
    std::size_t pos = json_text.find(needle);
    if (pos == std::string::npos) return {};
    pos = json_text.find(':', pos + needle.size());
    if (pos == std::string::npos) return {};
    ++pos;
    while (pos < json_text.size() &&
           (json_text[pos] == ' ' || json_text[pos] == '\t' || json_text[pos] == '\n' ||
            json_text[pos] == '\r')) {
        ++pos;
    }
    if (pos >= json_text.size() || json_text[pos] != '"') return {};
    ++pos;
    std::string value;
    while (pos < json_text.size()) {
        char c = json_text[pos];
        if (c == '\\' && pos + 1 < json_text.size()) {
            value += json_text[pos + 1];
            pos += 2;
            continue;
        }
        if (c == '"') break;
        value += c;
        ++pos;
    }
    return value;
}

}  // namespace sys
}  // namespace rplkit
