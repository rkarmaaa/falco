import { createApp } from 'vue';
import App from './App.vue';
import PillApp from './PillApp.vue';
import { i18n } from './i18n';
import './styles/main.scss';

// Stessa pagina per due finestre: l'app e il pulsantino a schermo intero
const root = location.hash === '#pill' ? PillApp : App;
createApp(root).use(i18n).mount('#app');
