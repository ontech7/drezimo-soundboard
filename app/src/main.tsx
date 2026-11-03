import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { guardTheWindow } from "./guard";
import "./styles.css";

guardTheWindow();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
