// Chiama un metodo del core e stampa la risposta. Rispetta FALCO_INSTANCE.
// Uso: node scripts/rpc.mjs <metodo> ['{"param":1}']
import { connect } from './lib/ipc.mjs';

const [method, raw] = process.argv.slice(2);
try {
  const c = await connect(process.env.FALCO_INSTANCE, { log: false });
  console.log(JSON.stringify(await c.call(method, raw ? JSON.parse(raw) : undefined)));
  c.close();
} catch (e) {
  console.error(`errore: ${e.message}`);
  process.exitCode = 1;
}
