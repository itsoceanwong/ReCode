import { useAppStore, type PageId } from "./store";
import Dashboard from "./pages/Dashboard";
import Tokens from "./pages/Tokens";
import { SettingsPage } from "./pages/Settings";
import { cn } from "./lib/utils";

const NAV: { id: PageId; label: string }[] = [
  { id: "dashboard", label: "Dashboard" },
  { id: "tokens", label: "Tokens" },
  { id: "settings", label: "Settings" },
];

function App() {
  const page = useAppStore((s) => s.page);
  const setPage = useAppStore((s) => s.setPage);

  return (
    <div className="flex h-full min-h-screen flex-col bg-[radial-gradient(ellipse_at_top,_#eef3f8_0%,_var(--color-background)_55%)]">
      <header className="border-b border-[var(--color-border)] bg-[var(--color-card)]/80 backdrop-blur">
        <div className="mx-auto flex max-w-6xl items-center justify-between gap-4 px-6 py-3">
          <div className="flex items-baseline gap-3">
            <span className="text-lg font-semibold tracking-tight text-[var(--color-primary)]">
              ReCode
            </span>
            <span className="text-xs text-[var(--color-muted-foreground)]">
              local agent monitor
            </span>
          </div>
          <nav className="flex gap-1">
            {NAV.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => setPage(item.id)}
                className={cn(
                  "rounded-md px-3 py-1.5 text-sm transition-colors",
                  page === item.id
                    ? "bg-[var(--color-primary)] text-[var(--color-primary-foreground)]"
                    : "text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)]",
                )}
              >
                {item.label}
              </button>
            ))}
          </nav>
        </div>
      </header>
      <main className="mx-auto w-full max-w-6xl flex-1 px-6 py-6">
        {page === "dashboard" && <Dashboard />}
        {page === "tokens" && <Tokens />}
        {page === "settings" && <SettingsPage />}
      </main>
    </div>
  );
}

export default App;
