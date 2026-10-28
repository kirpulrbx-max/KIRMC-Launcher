import React from "react";
import ReactDOM from "react-dom/client";
import "@fontsource/dela-gothic-one/400.css";
import "./styles/theme.css";
import "./styles/global.css";
import "./styles/pages.css";
import App from "./App";

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
