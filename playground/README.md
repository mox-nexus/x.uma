# x.uma Playground

Try the matcher engine in your browser: **https://mox-nexus.github.io/x.uma/playground/**

## Local Development

```bash
bun install    # from repo root (workspace)
cd playground
bun run dev    # http://localhost:5173
```

## Modes

**Config** — Write a `MatcherConfig` JSON + key-value context pairs. Evaluates via Registry.

**HTTP** — Write `HttpRouteMatch[]` JSON + method/path/headers. Evaluates via the compiler.

## Build

```bash
bun run build     # static output in build/
bun run preview   # serve the build locally
```

Uses `adapter-static` — pure client-side, no server.
