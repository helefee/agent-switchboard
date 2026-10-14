import React from "react";
import ReactDOM from "react-dom/client";
import "../styles/fonts.css";
import "../styles/tokens.css";
import "../styles/base.css";
import "./tray.css";
import { TrayPanel } from "./TrayPanel";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode><TrayPanel /></React.StrictMode>,
);
