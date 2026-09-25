#include "rplkit/native_tools.h"

#include <chrono>
#include <cctype>
#include <ctime>
#include <iomanip>
#include <random>
#include <sstream>
#include <stdexcept>

namespace rplkit {
namespace native {
namespace {

// --- Calculator: tokenizer + recursive descent -----------------------------

struct Token {
    enum Kind { Number, Plus, Minus, Star, Slash, SlashSlash, Percent, StarStar, LParen, RParen, End } kind;
    double value = 0;
};

class Lexer {
public:
    explicit Lexer(const std::string& s) : s_(s) {}

    Token next() {
        skip_ws();
        if (pos_ >= s_.size()) return Token{Token::End};
        char c = s_[pos_];
        if (std::isdigit(static_cast<unsigned char>(c)) || c == '.') return number();
        switch (c) {
            case '+': ++pos_; return Token{Token::Plus};
            case '-': ++pos_; return Token{Token::Minus};
            case '(': ++pos_; return Token{Token::LParen};
            case ')': ++pos_; return Token{Token::RParen};
            case '%': ++pos_; return Token{Token::Percent};
            case '*':
                if (pos_ + 1 < s_.size() && s_[pos_ + 1] == '*') {
                    pos_ += 2;
                    return Token{Token::StarStar};
                }
                ++pos_;
                return Token{Token::Star};
            case '/':
                if (pos_ + 1 < s_.size() && s_[pos_ + 1] == '/') {
                    pos_ += 2;
                    return Token{Token::SlashSlash};
                }
                ++pos_;
                return Token{Token::Slash};
            default:
                throw std::runtime_error(std::string("unexpected character: '") + c + "'");
        }
    }

private:
    const std::string& s_;
    std::size_t pos_ = 0;

    void skip_ws() {
        while (pos_ < s_.size() && std::isspace(static_cast<unsigned char>(s_[pos_]))) ++pos_;
    }

    Token number() {
        std::size_t start = pos_;
        bool dot_seen = false;
        while (pos_ < s_.size() &&
               (std::isdigit(static_cast<unsigned char>(s_[pos_])) || s_[pos_] == '.')) {
            if (s_[pos_] == '.') {
                if (dot_seen) break;
                dot_seen = true;
            }
            ++pos_;
        }
        double v = std::stod(s_.substr(start, pos_ - start));
        return Token{Token::Number, v};
    }
};

class Parser {
public:
    explicit Parser(const std::string& s) : lex_(s) { advance(); }

    double parse() {
        double v = expr();
        if (cur_.kind != Token::End) throw std::runtime_error("unexpected trailing input");
        return v;
    }

private:
    Lexer lex_;
    Token cur_{Token::End};

    void advance() { cur_ = lex_.next(); }

    double expr() {
        double v = term();
        while (cur_.kind == Token::Plus || cur_.kind == Token::Minus) {
            auto op = cur_.kind;
            advance();
            double rhs = term();
            v = (op == Token::Plus) ? v + rhs : v - rhs;
        }
        return v;
    }

    double term() {
        double v = factor();
        while (cur_.kind == Token::Star || cur_.kind == Token::Slash ||
               cur_.kind == Token::SlashSlash || cur_.kind == Token::Percent) {
            auto op = cur_.kind;
            advance();
            double rhs = factor();
            switch (op) {
                case Token::Star: v *= rhs; break;
                case Token::Slash:
                    if (rhs == 0) throw std::domain_error("division by zero");
                    v /= rhs;
                    break;
                case Token::SlashSlash: {
                    if (rhs == 0) throw std::domain_error("division by zero");
                    long long a = static_cast<long long>(v);
                    long long b = static_cast<long long>(rhs);
                    v = static_cast<double>(a / b);
                    break;
                }
                case Token::Percent: {
                    if (rhs == 0) throw std::domain_error("modulo by zero");
                    long long a = static_cast<long long>(v);
                    long long b = static_cast<long long>(rhs);
                    v = static_cast<double>(a % b);
                    break;
                }
                default: break;
            }
        }
        return v;
    }

    double factor() {
        double base = unary();
        if (cur_.kind == Token::StarStar) {  // right-associative
            advance();
            double exp = factor();
            double result = 1;
            long long n = static_cast<long long>(exp);
            if (static_cast<double>(n) != exp || n < 0) {
                // Non-integer exponent: fall back to repeated-squaring via exp/log.
                // Keep stdlib-only; use iterative multiplication for the common case.
                throw std::runtime_error("only non-negative integer exponents supported");
            }
            double b = base;
            while (n > 0) {
                if (n & 1) result *= b;
                b *= b;
                n >>= 1;
            }
            return result;
        }
        return base;
    }

    double unary() {
        if (cur_.kind == Token::Plus) {
            advance();
            return unary();
        }
        if (cur_.kind == Token::Minus) {
            advance();
            return -unary();
        }
        return primary();
    }

    double primary() {
        if (cur_.kind == Token::Number) {
            double v = cur_.value;
            advance();
            return v;
        }
        if (cur_.kind == Token::LParen) {
            advance();
            double v = expr();
            if (cur_.kind != Token::RParen) throw std::runtime_error("missing ')'");
            advance();
            return v;
        }
        throw std::runtime_error("expected number or '('");
    }
};

// --- Base64 ----------------------------------------------------------------

const char* kB64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

int b64val(char c) {
    if ('A' <= c && c <= 'Z') return c - 'A';
    if ('a' <= c && c <= 'z') return c - 'a' + 26;
    if ('0' <= c && c <= '9') return c - '0' + 52;
    if (c == '+') return 62;
    if (c == '/') return 63;
    return -1;
}

}  // namespace

double calc(const std::string& expression) {
    std::string t;
    for (char c : expression) {
        if (!std::isspace(static_cast<unsigned char>(c))) t += c;
    }
    if (t.empty()) throw std::runtime_error("empty expression");
    Parser p(t);
    return p.parse();
}

std::string b64encode(const std::string& bytes) {
    std::string out;
    for (std::size_t i = 0; i < bytes.size(); i += 3) {
        unsigned a = static_cast<unsigned char>(bytes[i]);
        unsigned b = (i + 1 < bytes.size()) ? static_cast<unsigned char>(bytes[i + 1]) : 0;
        unsigned c = (i + 2 < bytes.size()) ? static_cast<unsigned char>(bytes[i + 2]) : 0;
        unsigned triple = (a << 16) | (b << 8) | c;
        out += kB64[(triple >> 18) & 63];
        out += kB64[(triple >> 12) & 63];
        out += (i + 1 < bytes.size()) ? kB64[(triple >> 6) & 63] : '=';
        out += (i + 2 < bytes.size()) ? kB64[triple & 63] : '=';
    }
    return out;
}

std::string b64decode(const std::string& text) {
    std::string clean;
    for (char c : text) {
        if (!std::isspace(static_cast<unsigned char>(c))) clean += c;
    }
    if (clean.size() % 4 != 0) throw std::runtime_error("invalid base64 length");
    std::string out;
    for (std::size_t i = 0; i < clean.size(); i += 4) {
        int v[4];
        int pad = 0;
        for (int k = 0; k < 4; ++k) {
            char c = clean[i + k];
            if (c == '=') {
                v[k] = 0;
                ++pad;
            } else {
                v[k] = b64val(c);
                if (v[k] < 0) throw std::runtime_error("invalid base64 character");
            }
        }
        unsigned triple =
            (static_cast<unsigned>(v[0]) << 18) | (static_cast<unsigned>(v[1]) << 12) |
            (static_cast<unsigned>(v[2]) << 6) | static_cast<unsigned>(v[3]);
        out += static_cast<char>((triple >> 16) & 0xFF);
        if (pad < 2) out += static_cast<char>((triple >> 8) & 0xFF);
        if (pad < 1) out += static_cast<char>(triple & 0xFF);
    }
    return out;
}

std::string hex_encode(const std::string& bytes) {
    std::ostringstream ss;
    ss << std::hex << std::setfill('0');
    for (unsigned char c : bytes) ss << std::setw(2) << static_cast<unsigned>(c);
    return ss.str();
}

std::string hex_decode(const std::string& text) {
    auto val = [](char c) -> int {
        if ('0' <= c && c <= '9') return c - '0';
        if ('a' <= c && c <= 'f') return c - 'a' + 10;
        if ('A' <= c && c <= 'F') return c - 'A' + 10;
        return -1;
    };
    std::string t;
    for (char c : text) {
        if (!std::isspace(static_cast<unsigned char>(c))) t += c;
    }
    if (t.size() % 2 != 0) throw std::runtime_error("odd-length hex string");
    std::string out;
    for (std::size_t i = 0; i < t.size(); i += 2) {
        int hi = val(t[i]), lo = val(t[i + 1]);
        if (hi < 0 || lo < 0) throw std::runtime_error("invalid hex character");
        out += static_cast<char>((hi << 4) | lo);
    }
    return out;
}

std::string uuid4() {
    std::random_device rd;
    std::mt19937_64 gen(rd());
    std::uniform_int_distribution<unsigned long long> dist;
    unsigned long long hi = dist(gen), lo = dist(gen);
    unsigned char b[16];
    for (int i = 0; i < 8; ++i) b[i] = static_cast<unsigned char>((hi >> (8 * i)) & 0xFF);
    for (int i = 0; i < 8; ++i) b[8 + i] = static_cast<unsigned char>((lo >> (8 * i)) & 0xFF);
    b[6] = static_cast<unsigned char>((b[6] & 0x0F) | 0x40);  // version 4
    b[8] = static_cast<unsigned char>((b[8] & 0x3F) | 0x80);  // variant 10
    std::ostringstream ss;
    ss << std::hex << std::setfill('0');
    for (int i = 0; i < 16; ++i) {
        ss << std::setw(2) << static_cast<unsigned>(b[i]);
        if (i == 3 || i == 5 || i == 7 || i == 9) ss << "-";
    }
    return ss.str();
}

std::int64_t unix_now() {
    auto now = std::chrono::system_clock::now();
    return std::chrono::duration_cast<std::chrono::seconds>(now.time_since_epoch()).count();
}

std::string iso_now() {
    std::time_t t = static_cast<std::time_t>(unix_now());
    std::tm tm{};
#if defined(_WIN32)
    gmtime_s(&tm, &t);
#else
    gmtime_r(&t, &tm);
#endif
    std::ostringstream ss;
    ss << std::put_time(&tm, "%Y-%m-%dT%H:%M:%SZ");
    return ss.str();
}

}  // namespace native
}  // namespace rplkit
