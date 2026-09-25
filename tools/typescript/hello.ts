// hello — contoh minimal TypeScript tool untuk RPLKit.
//
// Kontrak: file .ts di tools/typescript/ otomatis ter-discovery oleh
// semua core (C++/Rust/Python). Dijalankan via `node <file> [args]`
// (node >= 22 men-strip tipe bawaan). Jaga file tetap bebas import
// agar bisa jalan tanpa `npm install`.
//
// Cara menambah tool baru: duplikat file ini, ganti logika run(),
// tambah <nama>.json berisi {"name","runtime":"typescript","description"}.

function run(args: string[]): number {
  const input = args.join(" ").trim();
  if (input === "") {
    console.log("hello from RPLKit TypeScript runtime. usage: hello <name>");
    return 0;
  }
  console.log(`hello, ${input}!`);
  return 0;
}

process.exit(run(process.argv.slice(2)));
