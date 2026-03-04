import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { Agentation } from 'agentation';

import './i18n';
import './styles/globals.css';
import { App } from './App';
import { QueryProvider } from './components/providers/QueryProvider';
import { ToastProvider } from './contexts/ToastContext';

const rootElement = document.getElementById('root');
if (!rootElement) {
  throw new Error('root element not found');
}

createRoot(rootElement).render(
  <StrictMode>
    <QueryProvider>
      <ToastProvider>
        <App />
        {import.meta.env.DEV && <Agentation endpoint="http://localhost:4747" />}
      </ToastProvider>
    </QueryProvider>
  </StrictMode>,
);
