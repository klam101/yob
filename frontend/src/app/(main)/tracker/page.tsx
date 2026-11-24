import { KanbanSquare } from "lucide-react";
import { Placeholder } from "@/components/shell/placeholder";

export const metadata = { title: "Tracker" };

export default function TrackerPage() {
  return (
    <Placeholder title="Tracker" icon={KanbanSquare}>
      Every job you saved, applied to or heard back from, grouped by stage.
    </Placeholder>
  );
}
