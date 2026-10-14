import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
// Bundled interface default (OFL): family "Noto Sans SC", the weights the
// token typography uses. Subsets load lazily via unicode-range.
import "./styles/fonts.css";
import "./styles/tokens.css";
import "./styles/base.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
