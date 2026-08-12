<script lang="ts">
  import type { EvalResult, ModeKind, Preset } from "$lib/playground/types.js";
  import { evaluateConfig, evaluateHttp } from "$lib/playground/engine.js";
  import { presets } from "$lib/playground/examples/index.js";
  import Editor from "$lib/playground/components/Editor.svelte";
  import ContextPanel from "$lib/playground/components/ContextPanel.svelte";
  import HttpContextPanel from "$lib/playground/components/HttpContextPanel.svelte";
  import ResultBadge from "$lib/playground/components/ResultBadge.svelte";
  import PresetPicker from "$lib/playground/components/PresetPicker.svelte";
  import ModeTabs from "$lib/playground/components/ModeTabs.svelte";
  import MatcherGraph from "$lib/playground/components/graph/MatcherGraph.svelte";

  // State
  let mode: ModeKind = $state("config");
  let activePresetId = $state(presets[0]!.id);
  let configJson = $state(presets[0]!.config);
  let context: Record<string, string> = $state(
    structuredClone(presets[0]!.context),
  );
  let httpMethod = $state("GET");
  let httpPath = $state("/");
  let httpHeaders: Record<string, string> = $state({});
  let result: EvalResult | null = $state(null);
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;
  let leftView: "code" | "graph" | "both" = $state("both");

  function loadPreset(preset: Preset) {
    activePresetId = preset.id;
    mode = preset.mode;
    configJson = preset.config;
    context = structuredClone(preset.context);
    if (preset.http) {
      httpMethod = preset.http.method;
      httpPath = preset.http.path;
      httpHeaders = structuredClone(preset.http.headers);
    } else {
      httpMethod = "GET";
      httpPath = "/";
      httpHeaders = {};
    }
    result = null;
    evaluate();
  }

  function evaluate() {
    if (mode === "config") {
      result = evaluateConfig(configJson, context);
    } else {
      result = evaluateHttp(configJson, httpMethod, httpPath, httpHeaders);
    }
  }

  function debouncedEvaluate() {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(evaluate, 300);
  }

  function onModeChange(newMode: ModeKind) {
    // Switch to first preset matching the new mode
    const matchingPreset = presets.find((p) => p.mode === newMode);
    if (matchingPreset) {
      loadPreset(matchingPreset);
    }
  }

  // Evaluate on mount
  $effect(() => {
    evaluate();
  });
</script>

<div class="container">
  <header class="header">
    <div class="title">
      <h1>x.uma <span class="subtitle">Playground</span> <span class="alpha-badge">alpha</span></h1>
    </div>
    <ModeTabs bind:mode onchange={onModeChange} />
  </header>

  <div class="main-layout">
    <div class="left-col">
      <PresetPicker
        {presets}
        active={activePresetId}
        onselect={loadPreset}
      />

      <div class="view-tabs">
        <button
          class="view-tab"
          class:active={leftView === "code"}
          onclick={() => (leftView = "code")}
        >Code</button>
        <button
          class="view-tab"
          class:active={leftView === "graph"}
          onclick={() => (leftView = "graph")}
        >Graph</button>
        <button
          class="view-tab"
          class:active={leftView === "both"}
          onclick={() => (leftView = "both")}
        >Both</button>
      </div>

      {#if leftView !== "graph"}
        <div class="editor-section">
          <div class="label">
            {mode === "config" ? "Matcher Config" : "Route Config"}
          </div>
          <Editor bind:value={configJson} oninput={debouncedEvaluate} />
        </div>
      {/if}

      {#if leftView !== "code"}
        <div class="graph-section">
          <div class="label">Matcher Tree</div>
          <MatcherGraph {configJson} {mode} />
        </div>
      {/if}

      <button class="eval-btn" onclick={evaluate}>Evaluate</button>
    </div>

    <div class="right-col">
      {#if mode === "config"}
        <ContextPanel
          bind:context
          onchange={debouncedEvaluate}
        />
      {:else}
        <HttpContextPanel
          bind:method={httpMethod}
          bind:path={httpPath}
          bind:headers={httpHeaders}
          onchange={debouncedEvaluate}
        />
      {/if}

      <ResultBadge {result} />
    </div>
  </div>

  <footer class="footer">
    <span class="muted">
      Powered by <a href="https://github.com/mox-nexus/x.uma" target="_blank" rel="noopener">xuma</a>
      &mdash; pure TypeScript, no WASM, no server.
    </span>
  </footer>
</div>

<style>
  .header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 24px;
  }

  .title h1 {
    font-size: 20px;
    font-weight: 700;
    color: var(--dao-text);
  }

  .subtitle {
    font-weight: 400;
    color: var(--dao-muted);
  }

  .alpha-badge {
    display: inline-block;
    font-size: 10px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    padding: 2px 6px;
    border-radius: 4px;
    background: color-mix(in srgb, var(--ev-weak) 15%, transparent);
    color: var(--ev-weak);
    border: 1px solid var(--ev-weak);
    vertical-align: middle;
    margin-left: 8px;
  }

  .main-layout {
    display: grid;
    grid-template-columns: 1fr 360px;
    gap: 20px;
    min-height: 0;
  }

  .left-col {
    display: flex;
    flex-direction: column;
    gap: 16px;
  }

  .right-col {
    display: flex;
    flex-direction: column;
    gap: 20px;
  }

  .view-tabs {
    display: flex;
    gap: 4px;
  }

  .view-tab {
    font-size: 12px;
    padding: 4px 10px;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--dao-muted);
    border: 1px solid var(--dao-border);
    cursor: pointer;
    transition: all 0.15s;
  }

  .view-tab.active {
    background: var(--dao-surface-2);
    color: var(--dao-text);
    border-color: var(--dao-muted);
  }

  .view-tab:hover:not(.active) {
    background: var(--dao-surface);
  }

  .editor-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .graph-section {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .eval-btn {
    align-self: flex-start;
    background: var(--ci-blue);
    color: var(--dao-bg);
    font-weight: 600;
    padding: 8px 20px;
    border-radius: var(--radius-sm);
    transition: opacity 0.15s;
  }

  .eval-btn:hover {
    opacity: 0.85;
  }

  .footer {
    margin-top: 32px;
    padding-top: 16px;
    border-top: 1px solid var(--dao-border);
  }

  .muted {
    font-size: 12px;
    color: var(--dao-muted);
  }

  .footer a {
    color: var(--ci-blue);
    text-decoration: none;
  }

  .footer a:hover {
    text-decoration: underline;
  }

  @media (max-width: 768px) {
    .main-layout {
      grid-template-columns: 1fr;
    }
  }
</style>
