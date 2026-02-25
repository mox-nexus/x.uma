<script lang="ts">
  import "../app.css";
  import type { EvalResult, ModeKind, Preset } from "$lib/types.js";
  import { evaluateConfig, evaluateHttp } from "$lib/engine.js";
  import { presets } from "$lib/examples/index.js";
  import Editor from "$lib/components/Editor.svelte";
  import ContextPanel from "$lib/components/ContextPanel.svelte";
  import HttpContextPanel from "$lib/components/HttpContextPanel.svelte";
  import ResultBadge from "$lib/components/ResultBadge.svelte";
  import PresetPicker from "$lib/components/PresetPicker.svelte";
  import ModeTabs from "$lib/components/ModeTabs.svelte";
  import MatcherGraph from "$lib/components/graph/MatcherGraph.svelte";

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
      <h1>x.uma <span class="subtitle">Playground</span></h1>
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
    color: var(--text);
  }

  .subtitle {
    font-weight: 400;
    color: var(--text-muted);
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
    border-radius: var(--radius);
    background: transparent;
    color: var(--text-muted);
    border: 1px solid var(--border);
    cursor: pointer;
    transition: all 0.15s;
  }

  .view-tab.active {
    background: var(--bg-elevated);
    color: var(--text);
    border-color: var(--text-muted);
  }

  .view-tab:hover:not(.active) {
    background: var(--bg-surface);
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
    background: var(--accent);
    color: var(--bg);
    font-weight: 600;
    padding: 8px 20px;
    border-radius: var(--radius);
    transition: opacity 0.15s;
  }

  .eval-btn:hover {
    opacity: 0.85;
  }

  .footer {
    margin-top: 32px;
    padding-top: 16px;
    border-top: 1px solid var(--border);
  }

  .muted {
    font-size: 12px;
    color: var(--text-muted);
  }

  .footer a {
    color: var(--accent);
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
