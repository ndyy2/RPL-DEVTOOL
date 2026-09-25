#pragma once

#include <cstdint>
#include <string>

namespace rplkit {
namespace native {

// Safe arithmetic: + - * / // % **, parentheses, unary +/-.
// Throws std::runtime_error on invalid input, std::domain_error on div-by-zero.
double calc(const std::string& expression);

std::string b64encode(const std::string& bytes);
std::string b64decode(const std::string& text);  // throws on invalid input

std::string hex_encode(const std::string& bytes);
std::string hex_decode(const std::string& text);  // throws on invalid input

std::string uuid4();        // random v4, e.g. 550e8400-e29b-41d4-a716-446655440000
std::int64_t unix_now();    // seconds since epoch (UTC)
std::string iso_now();      // UTC ISO-8601, e.g. 2026-09-24T18:52:00Z

}  // namespace native
}  // namespace rplkit
