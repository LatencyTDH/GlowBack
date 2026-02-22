import { Link, Outlet, useLocation } from "react-router-dom";

const NAV = [
  { to: "/backtests", label: "Backtests" },
  { to: "/backtests/new", label: "New Run" },
] as const;

export default function Layout() {
  const { pathname } = useLocation();
  return (
    <div className="min-h-screen bg-gray-50 text-gray-900">
      {/* Header */}
      <header className="bg-white border-b border-gray-200 shadow-sm">
        <div className="mx-auto flex max-w-7xl items-center gap-8 px-6 py-4">
          <Link to="/" className="flex items-center gap-2 text-xl font-bold text-emerald-600">
            <span className="text-2xl">🌟</span> GlowBack
          </Link>
          <nav className="flex gap-1">
            {NAV.map(({ to, label }) => {
              const active = pathname === to || (to === "/backtests" && pathname === "/");
              return (
                <Link
                  key={to}
                  to={to}
                  className={`rounded-md px-3 py-1.5 text-sm font-medium transition ${
                    active
                      ? "bg-emerald-50 text-emerald-700"
                      : "text-gray-600 hover:bg-gray-100 hover:text-gray-900"
                  }`}
                >
                  {label}
                </Link>
              );
            })}
          </nav>
        </div>
      </header>

      {/* Content */}
      <main className="mx-auto max-w-7xl px-6 py-8">
        <Outlet />
      </main>
    </div>
  );
}
