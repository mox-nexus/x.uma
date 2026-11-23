import{a as l,f as m,s as d}from"../chunks/DZ5l5Npp.js";import{i as U}from"../chunks/CLqXuRTp.js";import{p as H,s as a,f as K,a as L,c as t,n as F,$ as X,v as I,r as e,t as b,w as r,x as j}from"../chunks/Bpu2n9uS.js";import{e as w,s as z,a as B}from"../chunks/B6fv0Rl3.js";import{h as J}from"../chunks/Cmb1_t-t.js";import{b as V}from"../chunks/IrnQUEt0.js";import{M as Y}from"../chunks/DsdYmNzg.js";import{Q as T,b as Z}from"../chunks/D05jWaDr.js";var ee=m('<meta name="description" content="Match structured data against rule trees. Write the rules once, evaluate them in Rust, Python, or TypeScript, and get the same answer."/>'),te=m('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <pre class="svelte-1uha8ag"><code> </code></pre></article>'),ae=m("<li><a> </a></li>"),se=m('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <p class="svelte-1uha8ag"> </p> <ul class="svelte-1uha8ag"></ul></article>'),oe=m(`<section class="hero svelte-1uha8ag"><h1 class="svelte-1uha8ag">Write the rules once.<br/><em class="svelte-1uha8ag">Get the same answer everywhere.</em></h1> <p class="svelte-1uha8ag">x.uma is a matcher engine implementing the xDS Unified Matcher API. One config, one Rust
		engine, three languages, one conformance suite proving they agree.</p></section> <section class="demo svelte-1uha8ag"><h2 class="svelte-1uha8ag">It runs here</h2> <p class="lede svelte-1uha8ag">This is the real engine: rumi, compiled to WebAssembly and running in your browser, loading
		the config beside it. Change the method to <code>POST</code>, or to something else entirely,
		and watch the decision change.</p> <!> <p class="aside svelte-1uha8ag">Rules are tried in order and the first match wins. When nothing matches, <code>on_no_match</code> decides. That is the whole evaluation model.</p></section> <section class="runtimes svelte-1uha8ag"><h2 class="svelte-1uha8ag">The same config, three runtimes</h2> <p class="lede svelte-1uha8ag">The config above is data, not code. Every runtime hands it to the same engine and gets the
		same answer.</p> <div class="grid svelte-1uha8ag"></div></section> <section class="impls svelte-1uha8ag"><h2 class="svelte-1uha8ag">One engine, three runtimes</h2> <p class="lede svelte-1uha8ag">Pick by runtime. Every package is the same Rust engine; the bindings add a host-language
		surface, not a second implementation.</p> <table><thead><tr><th>Package</th><th>Language</th><th>What it is</th></tr></thead><tbody><tr><td><code>rumi-core</code></td><td>Rust</td><td>The engine. Reference implementation.</td></tr><tr><td><code>xuma-crust</code></td><td>Python 3.12+</td><td>The engine, bound through PyO3.</td></tr><tr><td><code>xuma-crust</code></td><td>TypeScript</td><td>The engine, compiled to WebAssembly.</td></tr></tbody></table> <p class="aside svelte-1uha8ag">All three run the same conformance fixtures from <code>spec/tests/</code>. A binding that
		disagrees with the engine fails its own build.</p></section> <section class="pipeline svelte-1uha8ag"><h2 class="svelte-1uha8ag">How a decision is made</h2> <pre><code></code></pre> <p class="lede svelte-1uha8ag">An <code>ExactMatcher</code> does not know whether it is matching an HTTP path, a Claude Code
		hook event, or your own domain. It matches <em>data</em>. Extracting that data from a context
		is a separate port, which is why one matcher works everywhere.</p></section> <section class="quadrants svelte-1uha8ag"><h2 class="svelte-1uha8ag">Where to go next</h2> <div class="grid svelte-1uha8ag"></div></section>`,1);function ge(S,N){H(N,!1);const D=["tutorial","how-to","reference","explanation"],W=`{
  "matcherList": {
    "matchers": [
      {
        "predicate": {
          "singlePredicate": {
            "input": {
              "name": "method",
              "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.MapInput", "key": "method" }
            },
            "valueMatch": { "exact": "GET" }
          }
        },
        "onMatch": {
          "action": {
            "name": "read-handler",
            "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "read-handler" }
          }
        }
      },
      {
        "predicate": {
          "singlePredicate": {
            "input": {
              "name": "method",
              "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.MapInput", "key": "method" }
            },
            "valueMatch": { "exact": "POST" }
          }
        },
        "onMatch": {
          "action": {
            "name": "write-handler",
            "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "write-handler" }
          }
        }
      }
    ]
  },
  "onNoMatch": {
    "action": {
      "name": "405-not-allowed",
      "typedConfig": { "@type": "type.googleapis.com/xuma.core.v1.NamedAction", "name": "405-not-allowed" }
    }
  }
}`,k='{ "method": "GET" }',$=[{lang:"Rust",code:`let matcher = registry.load_typed_matcher(config, &actions)?;
matcher.evaluate(&ctx)   // Some("read-handler")`},{lang:"Python",code:`matcher = KvMatcher.from_config(config)
matcher.evaluate(ctx)    # "read-handler"`},{lang:"TypeScript",code:`const matcher = KvMatcher.fromConfig(config);
matcher.evaluate(ctx);   // "read-handler"`}];U();var M=oe();J("1uha8ag",s=>{var o=ee();F(()=>{X.title="x.uma — cross-platform matcher engine"}),l(s,o)});var u=a(K(M),2),G=a(t(u),4);Y(G,{config:W,context:k}),I(2),e(u);var p=a(u,2),C=a(t(p),4);w(C,5,()=>$,s=>s.lang,(s,o)=>{var i=te(),n=t(i),h=t(n,!0);e(n);var g=a(n,2),c=t(g),f=t(c,!0);e(c),e(g),e(i),b(()=>{d(h,r(o).lang),d(f,r(o).code)}),l(s,i)}),e(C),e(p);var v=a(p,4),P=a(t(v),2),q=t(P);q.textContent=`Context  →  DataInput  →  MatchingData  →  InputMatcher  →  bool
            domain-        erased           domain-
            specific                        agnostic`,e(P),I(2),e(v);var A=a(v,2),E=a(t(A),2);w(E,5,()=>D,s=>s,(s,o)=>{const i=j(()=>Z(r(o)));var n=se(),h=t(n),g=t(h,!0);e(h);var c=a(h,2),f=t(c,!0);e(c);var R=a(c,2);w(R,5,()=>r(i).slice(0,4),y=>y.slug,(y,O)=>{var x=ae(),_=t(x),Q=t(_,!0);e(_),e(x),b(()=>{z(_,"href",`${V??""}/docs/${r(O).slug??""}`),d(Q,r(O).title)}),l(y,x)}),e(R),e(n),b(()=>{B(n,`--quadrant: ${T[r(o)].token??""}`),d(g,T[r(o)].label),d(f,T[r(o)].blurb)}),l(s,n)}),e(E),e(A),l(S,M),L()}export{ge as component};
