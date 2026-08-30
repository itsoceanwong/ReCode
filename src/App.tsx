import { useAppStore, type PageId } from "./store";
import Dashboard from "./pages/Dashboard";
import Tokens from "./pages/Tokens";
import Sessions from "./pages/Sessions";
import { SettingsPage } from "./pages/Settings";
import { cn } from "./lib/utils";
import {
  LayoutDashboard,
  Coins,
  Radio,
  Settings,
  Sun,
  Moon,
  Flame,
  Activity,
} from "lucide-react";

const NAV: { id: PageId; label: string; icon: React.ElementType }[] = [
  { id: "dashboard", label: "Dashboard", icon: LayoutDashboard },
  { id: "tokens", label: "Tokens", icon: Coins },
  { id: "sessions", label: "Sessions", icon: Radio },
  { id: "settings", label: "Settings", icon: Settings },
];

function App() {
  const page = useAppStore((s) => s.page);
  const setPage = useAppStore((s) => s.setPage);
  const resolvedTheme = useAppStore((s) => s.resolvedTheme);
  const toggleTheme = useAppStore((s) => s.toggleTheme);
  const isDark = resolvedTheme === "dark";

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-[var(--color-background)] text-[var(--color-foreground)]">
      {/* Modern Left Sidebar */}
      <aside className="flex w-60 flex-col justify-between border-r border-[var(--color-border)] bg-[var(--color-sidebar)] px-3.5 py-5 select-none shrink-0">
        <div className="space-y-6">
          {/* Logo Header */}
          <div className="flex items-center gap-3 px-2">
            <div className="flex h-9 w-9 items-center justify-center rounded-xl bg-gradient-to-br from-orange-500 to-amber-600 text-white shadow-md shadow-orange-500/20">
              <Flame className="h-5 w-5 fill-white/20" />
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="text-base font-bold tracking-tight text-[var(--color-foreground)]">
                  ReCode
                </span>
                <span className="rounded-full bg-orange-500/15 px-1.5 py-0.2 text-[10px] font-semibold text-orange-400">
                  v0.1
                </span>
              </div>
              <p className="text-[11px] text-[var(--color-muted-foreground)]">
                AI Agent Monitor
              </p>
            </div>
          </div>

          {/* Nav List */}
          <nav className="space-y-1">
            {NAV.map((item) => {
              const Icon = item.icon;
              const active = page === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setPage(item.id)}
                  className={cn(
                    "group relative flex w-full items-center gap-3 rounded-xl px-3.5 py-2.5 text-xs font-medium transition-all duration-150",
                    active
                      ? "bg-gradient-to-r from-orange-500/15 via-orange-500/5 to-transparent text-orange-400 font-semibold"
                      : "text-[var(--color-muted-foreground)] hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]",
                  )}
                >
                  {active && (
                    <span className="absolute left-0 h-5 w-1 rounded-r-full bg-orange-500 shadow-[0_0_8px_#f97316]" />
                  )}
                  <Icon
                    className={cn(
                      "h-4 w-4 transition-transform duration-150 group-hover:scale-110",
                      active ? "text-orange-500" : "text-[var(--color-muted-foreground)]",
                    )}
                  />
                  <span>{item.label}</span>
                </button>
              );
            })}
          </nav>
        </div>

        {/* Sidebar Footer */}
        <div className="space-y-3 pt-4 border-t border-[var(--color-border)] px-1">
          {/* Status Indicator */}
          <div className="flex items-center justify-between text-[11px] text-[var(--color-muted-foreground)]">
            <div className="flex items-center gap-2">
              <span className="relative flex h-2 w-2">
                <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-emerald-400 opacity-75"></span>
                <span className="relative inline-flex h-2 w-2 rounded-full bg-emerald-500"></span>
              </span>
              <span>Daemon Ready</span>
            </div>
            <Activity className="h-3.5 w-3.5 opacity-60" />
          </div>

          {/* Theme Toggle Button */}
          <button
            type="button"
            onClick={() => toggleTheme()}
            className="flex w-full items-center justify-between rounded-xl border border-[var(--color-border)] bg-[var(--color-card)] px-3 py-2 text-xs font-medium text-[var(--color-muted-foreground)] transition-colors hover:bg-[var(--color-muted)] hover:text-[var(--color-foreground)]"
          >
            <span className="flex items-center gap-2">
              {isDark ? <Moon className="h-3.5 w-3.5 text-orange-400" /> : <Sun className="h-3.5 w-3.5 text-amber-500" />}
              <span>{isDark ? "Dark Theme" : "Light Theme"}</span>
            </span>
            <span className="text-[10px] opacity-60">Switch</span>
          </button>
        </div>
      </aside>

      {/* Main Content Viewport - Fluid width for full screen */}
      <main className="flex-1 min-w-0 overflow-y-auto">
        <div className="w-full min-w-0 p-6 lg:p-8 space-y-6">
          {page === "dashboard" && <Dashboard />}
          {page === "tokens" && <Tokens />}
          {page === "sessions" && <Sessions />}
          {page === "settings" && <SettingsPage />}
        </div>
      </main>
    </div>
  );
}

export default App;
