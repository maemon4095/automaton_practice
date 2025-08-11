import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import tailwindcss from "@tailwindcss/vite";
import wasmPlugin from "vite-plugin-wasm";

// MEMO: deno infer wrong type and reports error deno-ts(2349).
// deno-lint-ignore no-explicit-any
const wasm: any = wasmPlugin;

export default defineConfig({
  plugins: [wasm(), solid(), tailwindcss()],
});
