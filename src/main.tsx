import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";
import { MemoryManager } from "./config/performance";

const memoryManager = MemoryManager.getInstance();
memoryManager.startCleanup();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
