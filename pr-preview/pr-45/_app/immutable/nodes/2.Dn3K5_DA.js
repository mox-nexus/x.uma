import{a as l,f as u,s as d}from"../chunks/C7IGVkAW.js";import{i as U}from"../chunks/CP6AjnRL.js";import{p as F,s as a,f as H,a as L,c as t,n as X,$ as j,v as k,r as e,t as b,w as o,x as z}from"../chunks/DYiHVAw2.js";import{e as T,s as B,a as J}from"../chunks/DcI0cIgz.js";import{h as K}from"../chunks/IoRnOvJ0.js";import{b as V}from"../chunks/dNJEApq0.js";import{M as Y}from"../chunks/lahDOZch.js";import{Q as w,b as Z}from"../chunks/CDwJ5Klk.js";var ee=u('<meta name="description" content="Match structured data against rule trees. Write the rules once, evaluate them in Rust, Python, or TypeScript, and get the same answer."/>'),te=u('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <pre class="svelte-1uha8ag"><code> </code></pre></article>'),ae=u("<li><a> </a></li>"),se=u('<article class="svelte-1uha8ag"><h3 class="svelte-1uha8ag"> </h3> <p class="svelte-1uha8ag"> </p> <ul class="svelte-1uha8ag"></ul></article>'),re=u(`<section class="hero svelte-1uha8ag"><h1 class="svelte-1uha8ag">Write the rules once.<br/><em class="svelte-1uha8ag">Get the same answer everywhere.</em></h1> <p class="svelte-1uha8ag">x.uma is a matcher engine implementing the xDS Unified Matcher API. One config, five
		implementations across three languages, one conformance suite proving they agree.</p></section> <section class="demo svelte-1uha8ag"><h2 class="svelte-1uha8ag">It runs here</h2> <p class="lede svelte-1uha8ag">This is the real engine. The pure TypeScript implementation, running in your browser, loading
		the config beside it. Change the method to <code>POST</code>, or to something else entirely,
		and watch the decision change.</p> <!> <p class="aside svelte-1uha8ag">Rules are tried in order and the first match wins. When nothing matches, <code>on_no_match</code> decides. That is the whole evaluation model.</p></section> <section class="runtimes svelte-1uha8ag"><h2 class="svelte-1uha8ag">The same config, three runtimes</h2> <p class="lede svelte-1uha8ag">The config above is data, not code. Every implementation loads it through a registry and
		evaluates it identically.</p> <div class="grid svelte-1uha8ag"></div></section> <section class="impls svelte-1uha8ag"><h2 class="svelte-1uha8ag">Five implementations</h2> <p class="lede svelte-1uha8ag">Pick by runtime and by how much speed you need. The pure implementations have no native
		dependency beyond RE2. The crusts are the Rust engine, bound.</p> <table><thead><tr><th>Package</th><th>Language</th><th>What it is</th></tr></thead><tbody><tr><td><code>rumi-core</code></td><td>Rust</td><td>The engine. Reference implementation.</td></tr><tr><td><code>xuma</code></td><td>Python 3.12+</td><td>Pure Python, RE2 for regex.</td></tr><tr><td><code>xuma</code></td><td>TypeScript</td><td>Pure TypeScript, RE2 for regex.</td></tr><tr><td><code>xuma-crust</code></td><td>Python</td><td>Rust via PyO3.</td></tr><tr><td><code>xuma-crust</code></td><td>TypeScript</td><td>Rust via WebAssembly.</td></tr></tbody></table> <p class="aside svelte-1uha8ag">All five run the same conformance fixtures from <code>spec/tests/</code>. An implementation
		that disagrees fails its own build.</p></section> <section class="pipeline svelte-1uha8ag"><h2 class="svelte-1uha8ag">How a decision is made</h2> <pre><code></code></pre> <p class="lede svelte-1uha8ag">An <code>ExactMatcher</code> does not know whether it is matching an HTTP path, a Claude Code
		hook event, or your own domain. It matches <em>data</em>. Extracting that data from a context
		is a separate port, which is why one matcher works everywhere.</p></section> <section class="quadrants svelte-1uha8ag"><h2 class="svelte-1uha8ag">Where to go next</h2> <div class="grid svelte-1uha8ag"></div></section>`,1);function me(A,C){F(C,!1);const D=["tutorial","how-to","reference","explanation"],W=`{
  "matchers": [
    {
      "predicate": {
        "type": "single",
        "input": { "type_url": "xuma.kv.v1.MapInput",
                   "config": { "key": "method" } },
        "value_match": { "Exact": "GET" }
      },
      "on_match": { "type": "action", "action": "read-handler" }
    },
    {
      "predicate": {
        "type": "single",
        "input": { "type_url": "xuma.kv.v1.MapInput",
                   "config": { "key": "method" } },
        "value_match": { "Exact": "POST" }
      },
      "on_match": { "type": "action", "action": "write-handler" }
    }
  ],
  "on_no_match": { "type": "action", "action": "405-not-allowed" }
}`,$='{ "method": "GET" }',G=[{lang:"Rust",code:`let matcher = registry.load_matcher(config)?;
matcher.evaluate(&ctx)   // Some("read-handler")`},{lang:"Python",code:`matcher = registry.load_matcher(config)
matcher.evaluate(ctx)    # "read-handler"`},{lang:"TypeScript",code:`const matcher = registry.loadMatcher(config);
matcher.evaluate(ctx);   // "read-handler"`}];U();var E=re();K("1uha8ag",s=>{var r=ee();X(()=>{j.title="x.uma — cross-platform matcher engine"}),l(s,r)});var p=a(H(E),2),N=a(t(p),4);Y(N,{config:W,context:$}),k(2),e(p);var v=a(p,2),R=a(t(v),4);T(R,5,()=>G,s=>s.lang,(s,r)=>{var i=te(),n=t(i),h=t(n,!0);e(n);var m=a(n,2),c=t(m),f=t(c,!0);e(c),e(m),e(i),b(()=>{d(h,o(r).lang),d(f,o(r).code)}),l(s,i)}),e(R),e(v);var g=a(v,4),P=a(t(g),2),q=t(P);q.textContent=`Context  →  DataInput  →  MatchingData  →  InputMatcher  →  bool
            domain-        erased           domain-
            specific                        agnostic`,e(P),k(2),e(g);var M=a(g,2),S=a(t(M),2);T(S,5,()=>D,s=>s,(s,r)=>{const i=z(()=>Z(o(r)));var n=se(),h=t(n),m=t(h,!0);e(h);var c=a(h,2),f=t(c,!0);e(c);var I=a(c,2);T(I,5,()=>o(i).slice(0,4),y=>y.slug,(y,O)=>{var _=ae(),x=t(_),Q=t(x,!0);e(x),e(_),b(()=>{B(x,"href",`${V??""}/docs/${o(O).slug??""}`),d(Q,o(O).title)}),l(y,_)}),e(I),e(n),b(()=>{J(n,`--quadrant: ${w[o(r)].token??""}`),d(m,w[o(r)].label),d(f,w[o(r)].blurb)}),l(s,n)}),e(S),e(M),l(A,E),L()}export{me as component};
