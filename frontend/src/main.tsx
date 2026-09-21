import { StrictMode, useState, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { ConfigProvider, theme, App } from "antd";
import AppComponent from "./App";
import ErrorBoundary from "./components/ErrorBoundary";

export function Root() {
  const [colorScheme, setColorScheme] = useState<"light" | "dark">(() => {
    const stored = localStorage.getItem("color-scheme");
    if (stored === "light" || stored === "dark") return stored;
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
  });

  useEffect(() => {
    const mql = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem("color-scheme")) {
        setColorScheme(e.matches ? "dark" : "light");
      }
    };
    mql.addEventListener("change", handler);
    return () => mql.removeEventListener("change", handler);
  }, []);

  return (
    <StrictMode>
      <ConfigProvider
        theme={{
          algorithm:
            colorScheme === "dark"
              ? theme.darkAlgorithm
              : theme.defaultAlgorithm,
        }}
      >
        <App>
          <ErrorBoundary>
            <AppComponent
              colorScheme={colorScheme}
              setColorScheme={setColorScheme}
            />
          </ErrorBoundary>
        </App>
      </ConfigProvider>
    </StrictMode>
  );
}

createRoot(document.getElementById("root")!).render(<Root />);
