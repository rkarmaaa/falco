// Installer dal repository privato: il Worker trova l'ultima release con un token di sola lettura
// e rimanda al link firmato e temporaneo di GitHub. Il file (50+ MB) non passa dal Worker.

const API = 'https://api.github.com';

interface Asset {
  url: string;
  name: string;
}

export async function installerUrl(repo: string, token: string): Promise<string | null> {
  const headers = { authorization: `Bearer ${token}`, accept: 'application/vnd.github+json', 'user-agent': 'falco-hub' };
  const r = await fetch(`${API}/repos/${repo}/releases?per_page=10`, { headers });
  if (!r.ok) throw new Error(`releases ${r.status}`);
  const releases = (await r.json()) as { draft: boolean; assets: Asset[] }[];
  const asset = releases.filter((x) => !x.draft).flatMap((x) => x.assets).find((a) => /_x64-setup\.exe$/.test(a.name));
  if (!asset) return null;
  // Senza seguire il redirect: il link firmato vale qualche minuto e non vuole il token
  const file = await fetch(asset.url, { headers: { ...headers, accept: 'application/octet-stream' }, redirect: 'manual' });
  return file.headers.get('location');
}
