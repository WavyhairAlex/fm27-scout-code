import { StaffFilterSidebar } from "./StaffFilterSidebar";
import { StaffInspector } from "./StaffInspector";
import { StaffTable } from "./StaffTable";
import { useAppStore } from "@/store/app";

export function StaffView() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const inspectorOpen = useAppStore((state) => state.inspectorOpen);

  return (
    <>
      {sidebarOpen && <StaffFilterSidebar />}
      <StaffTable />
      {inspectorOpen && <StaffInspector />}
    </>
  );
}
