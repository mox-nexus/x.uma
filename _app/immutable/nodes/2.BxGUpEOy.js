import{a as l,f as m,s as d}from"../chunks/C7IGVkAW.js";import{i as U}from"../chunks/CP6AjnRL.js";import{p as F,s as a,f as H,a as L,c as t,n as X,$ as j,v as I,r as e,t as b,w as r,x as z}from"../chunks/DYiHVAw2.js";import{e as T,s as B,a as J}from"../chunks/DcI0cIgz.js";import{h as K}from"../chunks/IoRnOvJ0.js";import{b as V}from"../chunks/BD745tM4.js";import{M as Y}from"../chunks/Y8z-qYU3.js";import{Q as w,b as Z}from"../chunks/CDwJ5Klk.js";var ee=m('<meta name="description" content="Match structured data against rule trees. Write the rules once, evaluate them in Rust, Python, or TypeScript, and get the same answer."/>'),te=m('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <pre class="svelte-1uha8ag"><code> </code></pre></article>'),ae=m("<li><a> </a></li>"),se=m('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <p class="svelte-1uha8ag"> </p> <ul class="svelte-1uha8ag"></ul></article>'),oe=m(`<section class="hero svelte-1uha8ag"><h1 class="svelte-1uha8ag">Write the rules once.<br/><em class="svelte-1uha8ag">Get the same answer everywhere.</em></h1> <p class="svelte-1uha8ag">x.uma is a matcher engine implementing the xDS Unified Matcher API. One config, five
		implementations across three languages, one conformance suite proving they agree.</p></section> <section class="demo svelte-1uha8ag"><h2 class="svelte-1uha8ag">It runs here</h2> <p class="lede svelte-1uha8ag">This is the real engine. The pure TypeScript implementation, running in your browser, loading
		the config beside it. Change the method to <code>POST</code>, or to something else entirely,
		and watch the decision change.</p> <!> <p class="aside svelte-1uha8ag">Rules are tried in order and the first match wins. When nothing matches, <code>on_no_match</code> decides. That is the whole evaluation model.</p></section> <section class="runtimes svelte-1uha8ag"><h2 class="svelte-1uha8ag">The same config, three runtimes</h2> <p class="lede svelte-1uha8ag">The config above is data, not code. Every implementation loads it through a registry and
		evaluates it identically.</p> <div class="grid svelte-1uha8ag"></div></section> <section class="impls svelte-1uha8ag"><h2 class="svelte-1uha8ag">Five implementations</h2> <p class="lede svelte-1uha8ag">Pick by runtime and by how much speed you need. The pure implementations have no native
		dependency beyond RE2. The crusts are the Rust engine, bound.</p> <table><thead><tr><th>Package</th><th>Language</th><th>What it is</th></tr></thead><tbody><tr><td><code>rumi-core</code></td><td>Rust</td><td>The engine. Reference implementation.</td></tr><tr><td><code>xuma</code></td><td>Python 3.12+</td><td>Pure Python, RE2 for regex.</td></tr><tr><td><code>xuma</code></td><td>TypeScript</td><td>Pure TypeScript, RE2 for regex.</td></tr><tr><td><code>xuma-crust</code></td><td>Python</td><td>Rust via PyO3.</td></tr><tr><td><code>xuma-crust</code></td><td>TypeScript</td><td>Rust via WebAssembly.</td></tr></tbody></table> <p class="aside svelte-1uha8ag">All five run the same conformance fixtures from <code>spec/tests/</code>. An implementation
		that disagrees fails its own build.</p></section> <section class="pipeline svelte-1uha8ag"><h2 class="svelte-1uha8ag">How a decision is made</h2> <pre><code></code></pre> <p class="lede svelte-1uha8ag">An <code>ExactMatcher</code> does not know whether it is matching an HTTP path, a Claude Code
		hook event, or your own domain. It matches <em>data</em>. Extracting that data from a context
		is a separate port, which is why one matcher works everywhere.</p></section> <section class="quadrants svelte-1uha8ag"><h2 class="svelte-1uha8ag">Where to go next</h2> <div class="grid svelte-1uha8ag"></div></section>`,1);function ue(O,N){F(N,!1);const k=["tutorial","how-to","reference","explanation"],D=`{
  "matcherList": {
    "matchers": [
      {
        "predicate": {
          "singlePredicate": {
            "input": {
              "name": "method",
              "typedConfig": { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "method" }
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
              "typedConfig": { "@type": "type.googleapis.com/xuma.kv.v1.MapInput", "key": "method" }
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
}`,W='{ "method": "GET" }',$=[{lang:"Rust",code:`let matcher = registry.load_matcher(config)?;
matcher.evaluate(&ctx)   // Some("read-handler")`},{lang:"Python",code:`matcher = registry.load_matcher(config)
matcher.evaluate(ctx)    # "read-handler"`},{lang:"TypeScript",code:`const matcher = registry.loadMatcher(config);
matcher.evaluate(ctx);   // "read-handler"`}];U();var M=oe();K("1uha8ag",s=>{var o=ee();X(()=>{j.title="x.uma — cross-platform matcher engine"}),l(s,o)});var p=a(H(M),2),G=a(t(p),4);Y(G,{config:D,context:W}),I(2),e(p);var g=a(p,2),P=a(t(g),4);T(P,5,()=>$,s=>s.lang,(s,o)=>{var i=te(),n=t(i),h=t(n,!0);e(n);var u=a(n,2),c=t(u),y=t(c,!0);e(c),e(u),e(i),b(()=>{d(h,r(o).lang),d(y,r(o).code)}),l(s,i)}),e(P),e(g);var v=a(g,4),R=a(t(v),2),q=t(R);q.textContent=`Context  →  DataInput  →  MatchingData  →  InputMatcher  →  bool
            domain-        erased           domain-
            specific                        agnostic`,e(R),I(2),e(v);var E=a(v,2),C=a(t(E),2);T(C,5,()=>k,s=>s,(s,o)=>{const i=z(()=>Z(r(o)));var n=se(),h=t(n),u=t(h,!0);e(h);var c=a(h,2),y=t(c,!0);e(c);var S=a(c,2);T(S,5,()=>r(i).slice(0,4),f=>f.slug,(f,A)=>{var x=ae(),_=t(x),Q=t(_,!0);e(_),e(x),b(()=>{B(_,"href",`${V??""}/docs/${r(A).slug??""}`),d(Q,r(A).title)}),l(f,x)}),e(S),e(n),b(()=>{J(n,`--quadrant: ${w[r(o)].token??""}`),d(u,w[r(o)].label),d(y,w[r(o)].blurb)}),l(s,n)}),e(C),e(E),l(O,M),L()}export{ue as component};
