import { Settings as SettingsIcon } from "lucide-react";
import { Placeholder } from "@/components/shell/placeholder";

export const metadata = { title: "Settings" };

export default function SettingsPage() {
  return (
    <Placeholder title="Settings" icon={SettingsIcon}>
      Account, AI models and job sources.
    </Placeholder>
  );
}
