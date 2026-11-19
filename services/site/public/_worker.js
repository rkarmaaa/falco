// Ingresso di falcoapp.pages.dev: tutto va al Worker falco-hub, che usa questo dominio nei link.
export default {
  fetch: (request, env) => env.HUB.fetch(request),
};
