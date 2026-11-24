import { FileText } from "lucide-react";
import { Placeholder } from "@/components/shell/placeholder";

export const metadata = { title: "Documents" };

export default function DocumentsPage() {
  return (
    <Placeholder title="Documents" icon={FileText}>
      Tailored resumes and cover letters for each job, with a keyword gap report.
    </Placeholder>
  );
}
