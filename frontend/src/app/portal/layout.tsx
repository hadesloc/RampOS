import { Metadata } from "next";
import "../[locale]/globals.css";
import Providers from "@/components/providers";

export const metadata: Metadata = {
    title: "RampOS Portal",
    description: "User portal for RampOS",
};

export default function PortalLayout({
    children,
}: {
    children: React.ReactNode;
}) {
    return (
        <html lang="vi">
            <body>
                <Providers>
                    <main className="min-h-screen bg-background">
                        {children}
                    </main>
                </Providers>
            </body>
        </html>
    );
}
