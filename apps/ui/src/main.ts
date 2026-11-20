import { createApp } from 'vue';
import App from './App.vue';
import PillApp from './PillApp.vue';
import ToastsApp from './ToastsApp.vue';
import { i18n } from './i18n';
import './styles/main.scss';

// Stessa pagina per tre finestre: l'app, il pulsantino a schermo intero e le notifiche sopra lo stream
const root = location.hash === '#pill' ? PillApp : location.hash === '#toasts' ? ToastsApp : App;
createApp(root).use(i18n).mount('#app');
