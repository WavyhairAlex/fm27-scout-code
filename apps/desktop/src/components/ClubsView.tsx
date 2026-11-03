import { ClubFilterSidebar } from "./ClubFilterSidebar";
import { ClubInspector } from "./ClubInspector";
import { ClubTable } from "./ClubTable";
import { useAppStore } from "@/store/app";

export function ClubsView() {
  const sidebarOpen = useAppStore((state) => state.sidebarOpen);
  const inspectorOpen = useAppStore((state) => state.inspectorOpen);

  return (
    <>
      {sidebarOpen && <ClubFilterSidebar />}
      <ClubTable />
      {inspectorOpen && <ClubInspector />}
    </>
  );
}
