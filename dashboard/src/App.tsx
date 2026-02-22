import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import Layout from "./components/Layout";
import BacktestList from "./pages/BacktestList";
import NewBacktest from "./pages/NewBacktest";
import BacktestDetail from "./pages/BacktestDetail";

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route element={<Layout />}>
          <Route index element={<Navigate to="/backtests" replace />} />
          <Route path="backtests" element={<BacktestList />} />
          <Route path="backtests/new" element={<NewBacktest />} />
          <Route path="backtests/:runId" element={<BacktestDetail />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}
