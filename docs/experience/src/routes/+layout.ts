// The site is fully static: content is read at build time and there is no
// server. adapter-static requires every route to be prerenderable.
export const prerender = true;
export const trailingSlash = 'always';
