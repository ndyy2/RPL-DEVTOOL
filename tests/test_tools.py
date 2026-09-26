"""Unit tests — stdlib unittest agar jalan tanpa pytest.

Jalankan:  python3 -m unittest discover -s tests -v
"""

import json
import math
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


class CalculatorTest(unittest.TestCase):
    def test_basic(self):
        from tools.python.calculator import calculate

        self.assertEqual(calculate("2 + 3 * 4"), 14)
        self.assertEqual(calculate("(2 + 3) * 4"), 20)
        self.assertEqual(calculate("2 ** 10"), 1024)
        self.assertAlmostEqual(calculate("7 / 2"), 3.5)

    def test_rejects_code(self):
        from tools.python.calculator import calculate

        for bad in ("", "__import__('os')", "open('f')", "1/0", "2 +"):
            with self.assertRaises((ValueError, ZeroDivisionError), msg=bad):
                calculate(bad)


class ConverterTest(unittest.TestCase):
    def test_temp_length_weight(self):
        from tools.python.converter import convert

        self.assertAlmostEqual(convert(100, "C", "F"), 212)
        self.assertAlmostEqual(convert(32, "F", "C"), 0)
        self.assertAlmostEqual(convert(1, "km", "m"), 1000)
        self.assertAlmostEqual(convert(1, "kg", "g"), 1000)
        with self.assertRaises(ValueError):
            convert(1, "km", "kg")


class EncoderTest(unittest.TestCase):
    def test_roundtrips(self):
        from tools.python import encoder as e

        self.assertEqual(e.b64_decode(e.b64_encode("halo★")), "halo★")
        self.assertEqual(e.hex_decode(e.hex_encode("halo")), "halo")
        self.assertEqual(e.url_decode(e.url_encode("a b/c?")), "a b/c?")
        self.assertEqual(e.rot13(e.rot13("abc")), "abc")
        with self.assertRaises(ValueError):
            e.b64_decode("!!!")


class JsonFormatterTest(unittest.TestCase):
    def test_format_and_validate(self):
        from tools.python.json_formatter import format_json, validate_json

        out = format_json('{"b":1,"a":2}', sort_keys=True)
        self.assertEqual(json.loads(out), {"a": 2, "b": 1})
        self.assertTrue(validate_json("[1,2]"))
        self.assertFalse(validate_json("{oops"))
        with self.assertRaises(ValueError):
            format_json("{oops")


class UuidTimestampTest(unittest.TestCase):
    def test_uuid(self):
        from tools.python.uuid_generator import generate

        ids = generate(3)
        self.assertEqual(len(ids), 3)
        self.assertEqual(len({len(i) for i in ids}), 1)
        with self.assertRaises(ValueError):
            generate(0)

    def test_timestamp(self):
        from tools.python.timestamp import iso_to_unix, now_iso, now_unix, unix_to_iso

        self.assertGreater(now_unix(), 1_700_000_000)
        self.assertTrue(now_iso().endswith("Z"))
        self.assertEqual(unix_to_iso(0), "1970-01-01T00:00:00Z")
        self.assertEqual(int(iso_to_unix("1970-01-01T00:00:00Z")), 0)
        with self.assertRaises(ValueError):
            iso_to_unix("bukan-tanggal")


class FileUtilsTest(unittest.TestCase):
    def test_size_hash_ls_human(self):
        from tools.python import file_utils as fu

        with tempfile.TemporaryDirectory() as d:
            p = os.path.join(d, "a.txt")
            with open(p, "w") as f:
                f.write("abc")
            self.assertEqual(fu.file_size(p), 3)
            self.assertEqual(fu.sha256(p), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
            self.assertIn("a.txt", fu.list_dir(d))
        self.assertEqual(fu.human_size(1024), "1.0 KiB")
        self.assertEqual(fu.human_size(0), "0 B")
        with self.assertRaises(FileNotFoundError):
            fu.file_size(os.path.join(d, "hilang"))
        with self.assertRaises(ValueError):
            fu.human_size(-1)


class QrBoxTest(unittest.TestCase):
    def test_box(self):
        from tools.python.qr_generator import qr_ascii_box

        box = qr_ascii_box("hi")
        self.assertIn("hi", box)
        self.assertTrue(box.startswith("+"))
        with self.assertRaises(ValueError):
            qr_ascii_box("hi", width=5)


class RegistryTest(unittest.TestCase):
    def test_discovery(self):
        from tools import discover

        tools = discover()
        names = {t.name for t in tools}
        for expect in ("calculator", "converter", "encoder", "json_formatter",
                       "uuid_generator", "timestamp", "qr_generator", "file_utils", "hello",
                       "system-info", "process", "memory", "disk", "port-check", "dns",
                       "git-clean", "git-status", "project-init"):
            self.assertIn(expect, names, expect)
        runtimes = {t.name: t.runtime for t in tools}
        self.assertEqual(runtimes["calculator"], "python")
        self.assertEqual(runtimes["hello"], "typescript")
        modules = {t.name: t.module for t in tools}
        self.assertEqual(modules["system-info"], "system-tools")
        self.assertEqual(modules["dns"], "network-tools")
        self.assertEqual(modules["git-status"], "git-tools")
        self.assertEqual(modules["calculator"], "dev-tools")

    def test_menu_compat(self):
        from core.menu import run_menu

        self.assertTrue(callable(run_menu))

    def test_native_cli(self):
        import subprocess

        for exe in ("build/cpp/rplkit", "core/rust/target/debug/rplkit"):
            if os.path.isfile(exe) and os.access(exe, os.X_OK):
                out = subprocess.run([exe, "--run", "calculator", "2+2"], capture_output=True,
                                     text=True, timeout=20)
                self.assertEqual(out.stdout.strip(), "4", exe)
                break


class BridgeTest(unittest.TestCase):
    def test_bridge_matches_local_discovery(self):
        from modules import manifests, rust_bridge

        root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        bridged = rust_bridge.list_via_bin(root)
        if bridged is None:
            self.skipTest("biner Rust belum di-build")
        local = manifests.discover(root)
        self.assertEqual({t.name for t in bridged}, {t.name for t in local})

    def test_bridge_fallback_without_binary(self):
        import tempfile
        from modules import rust_bridge

        with tempfile.TemporaryDirectory() as d:
            # Tanpa core/rust/target → None → pemanggil pakai lokal.
            self.assertIsNone(rust_bridge.list_via_bin(d))
            self.assertIsNone(rust_bridge.find_bin(d))

    def test_module_cache_reuses_import(self):
        from modules import python_runtime

        python_runtime.clear_cache()
        mod1 = python_runtime.load(_meta("calculator"))
        mod2 = python_runtime.load(_meta("calculator"))
        self.assertIs(mod1, mod2)
        self.assertIn("tools/python/calculator.py", python_runtime._module_cache)
        python_runtime.clear_cache()
        self.assertEqual(python_runtime._module_cache, {})


def _meta(name):
    from modules import ToolMeta

    return ToolMeta(name=name, runtime="python", entry=f"tools/python/{name}.py")


class ConverterEdgeTest(unittest.TestCase):
    def test_kelvin_identity(self):
        from tools.python.converter import convert_temperature

        self.assertAlmostEqual(convert_temperature(25, "C", "C"), 25)
        self.assertAlmostEqual(convert_temperature(0, "C", "K"), 273.15)


class ProjectInitTest(unittest.TestCase):
    def test_scaffold_python(self):
        import tempfile
        from tools.python.project_init import scaffold

        with tempfile.TemporaryDirectory() as d:
            created = scaffold("demo", "python", d)
            self.assertTrue(any(p.endswith("demo/main.py") for p in created))
            self.assertTrue(os.path.isfile(os.path.join(d, "demo", "requirements.txt")))

    def test_rejects_bad_input(self):
        import tempfile
        from tools.python.project_init import scaffold

        with tempfile.TemporaryDirectory() as d:
            with self.assertRaises(ValueError):
                scaffold("demo", "rust", d)
            with self.assertRaises(ValueError):
                scaffold("", "python", d)


if __name__ == "__main__":
    unittest.main()
