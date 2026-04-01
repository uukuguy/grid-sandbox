import { useAtom } from "jotai";
import { AppLayout } from "./components/layout/AppLayout";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { ToastContainer } from "./components/Toast";
import { activeTabAtom } from "./atoms/ui";
import Chat from "./pages/Chat";
import Tasks from "./pages/Tasks";
import Schedule from "./pages/Schedule";
import Tools from "./pages/Tools";
import Memory from "./pages/Memory";
import Debug from "./pages/Debug";
import McpWorkbench from "./pages/McpWorkbench";
import Collaboration from "./pages/Collaboration";

export default function App() {
  const [activeTab] = useAtom(activeTabAtom);

  return (
    <ErrorBoundary>
      <AppLayout>
        {activeTab === "chat" && <Chat />}
        {activeTab === "tasks" && <Tasks />}
        {activeTab === "schedule" && <Schedule />}
        {activeTab === "tools" && <Tools />}
        {activeTab === "memory" && <Memory />}
        {activeTab === "debug" && <Debug />}
        {activeTab === "mcp" && <McpWorkbench />}
        {activeTab === "collaboration" && <Collaboration />}
      </AppLayout>
      <ToastContainer />
    </ErrorBoundary>
  );
}
