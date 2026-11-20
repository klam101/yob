import { UserRound } from "lucide-react";
import { Placeholder } from "@/components/shell/placeholder";

export const metadata = { title: "Profile" };

export default function ProfilePage() {
  return (
    <Placeholder title="Profile" icon={UserRound}>
      Your search preferences from the questionnaire, plus experience, education and skills parsed from your resume.
    </Placeholder>
  );
}
