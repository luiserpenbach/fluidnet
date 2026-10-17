import React from 'react';
import { createRoot } from 'react-dom/client';
import '@xyflow/react/dist/style.css';
import 'uplot/dist/uPlot.min.css';
import './styles.css';
import App from './App';

createRoot(document.getElementById('root')!).render(<App />);
