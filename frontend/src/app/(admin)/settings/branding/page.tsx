import { Metadata } from "next";
import { ThemeCustomizer } from "@/components/theme/theme-customizer";
import { PageHeader } from "@/components/layout/page-header";
import { WhiteLabelProvider } from "@/lib/theme/provider";

export const metadata: Metadata = {
  title: "Branding & Theming - RampOS Admin",
  description: "Customize the look and feel of your RampOS instance.",
};

export default function BrandingPage() {
  return (
    <WhiteLabelProvider>
      <div className="flex flex-col h-[calc(100vh-4rem)]">
        <div className="flex-none p-6 pb-0">
          <PageHeader
            title="Branding & Theming"
            description="Customize your brand identity, colors, and logos. Changes apply instantly to your tenant."
          />
        </div>
        <div className="flex-1 min-h-0 p-6">
          <ThemeCustomizer />
        </div>
      </div>
    </WhiteLabelProvider>
  );
}
