import React from "react";
import ReactDOM from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import App from "./App";
import "./styles.css";

// Suppress the webview context menu on non-editable content so the UI behaves
// like a native app. Text fields keep their menu for cut/copy/paste.
document.addEventListener("contextmenu", (event) => {
  const target = event.target;
  if (
    target instanceof Element &&
    target.closest("input, textarea, [contenteditable='true']")
  ) {
    return;
  }
  event.preventDefault();
});

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      retry: 1,
      refetchOnWindowFocus: false,
    },
  },
});

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}>
      <App />
    </QueryClientProvider>
  </React.StrictMode>,
);
