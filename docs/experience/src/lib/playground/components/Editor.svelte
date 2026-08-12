<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { EditorView, basicSetup } from "codemirror";
  import { json } from "@codemirror/lang-json";
  import { oneDark } from "@codemirror/theme-one-dark";
  import { EditorState } from "@codemirror/state";

  let {
    value = $bindable(""),
    oninput,
  }: {
    value: string;
    oninput?: (value: string) => void;
  } = $props();

  let container: HTMLDivElement;
  let view: EditorView | undefined;
  let ignoreNextUpdate = false;

  onMount(() => {
    const updateListener = EditorView.updateListener.of((update) => {
      if (update.docChanged) {
        ignoreNextUpdate = true;
        const doc = update.state.doc.toString();
        value = doc;
        oninput?.(doc);
      }
    });

    view = new EditorView({
      state: EditorState.create({
        doc: value,
        extensions: [
          basicSetup,
          json(),
          oneDark,
          updateListener,
          EditorView.theme({
            "&": {
              fontSize: "13px",
              maxHeight: "400px",
            },
            ".cm-scroller": {
              overflow: "auto",
              fontFamily:
                "'JetBrains Mono', 'Fira Code', 'SF Mono', Menlo, monospace",
            },
            ".cm-gutters": {
              backgroundColor: "transparent",
              borderRight: "1px solid var(--dao-border)",
            },
          }),
        ],
      }),
      parent: container,
    });
  });

  // Sync external value changes into the editor
  $effect(() => {
    if (ignoreNextUpdate) {
      ignoreNextUpdate = false;
      return;
    }
    if (view && view.state.doc.toString() !== value) {
      view.dispatch({
        changes: {
          from: 0,
          to: view.state.doc.length,
          insert: value,
        },
      });
    }
  });

  onDestroy(() => {
    view?.destroy();
  });
</script>

<div class="editor-wrap" bind:this={container}></div>

<style>
  .editor-wrap {
    border: 1px solid var(--dao-border);
    border-radius: var(--radius-sm);
    overflow: hidden;
  }

  .editor-wrap :global(.cm-editor) {
    background: var(--dao-bg);
  }

  .editor-wrap :global(.cm-focused) {
    outline: 1px solid var(--ci-blue);
  }
</style>
