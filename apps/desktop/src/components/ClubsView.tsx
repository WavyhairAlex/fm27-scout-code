import { ClubFilterBar } from "./ClubFilterBar";
import { ClubInspector } from "./ClubInspector";
import { ClubTable } from "./ClubTable";

export function ClubsView() {
  return (
    <div className="flex min-h-0 min-w-0 flex-1 flex-col overflow-hidden">
      <ClubFilterBar />
      <div className="flex min-h-0 min-w-0 flex-1 overflow-hidden">
        <ClubTable />
        <ClubInspector />
      </div>
    </div>
  );
}
