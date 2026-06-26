import type { Configuration } from "lint-staged";

const cargoBuild = "cargo build";

export default {
  "schema.yaml": () => cargoBuild,
  "Cargo.(lock,toml)": () => cargoBuild,
  "*.rs": (files) => [cargoBuild, `cargo fmt -- ${files.join(" ")}`],
  "*.{yaml,json,ts}": "npx prettier --write",
} satisfies Configuration;
