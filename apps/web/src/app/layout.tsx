import type { Metadata } from "next";
import { Geist, Geist_Mono } from "next/font/google";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { WalletProvider } from "@/providers/WalletProvider";
import { NetworkSecurityBanner } from "@/components/wallet/NetworkSecurityBanner";
import { BackgroundPattern } from "@/components/ui/BackgroundPattern";
import { FloatingScrollbar } from "@/components/ui/FloatingScrollbar";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "CanguPay // B2B Conditional Escrow",
  description: "Conditional B2B escrow and settlement platform on Stellar Soroban",
};

const themeScript = `(function() {
  try {
    var stored = localStorage.getItem('cangupay-theme') || 'system';
    var isDark = stored === 'dark' || (stored === 'system' && window.matchMedia('(prefers-color-scheme: dark)').matches);
    if (isDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  } catch (e) {}
})();`;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="es"
      suppressHydrationWarning
      className={`${geistSans.variable} ${geistMono.variable} h-full antialiased`}
    >
      <head>
        <script dangerouslySetInnerHTML={{ __html: themeScript }} />
      </head>
      <body className="min-h-full flex flex-col bg-neutral-50 dark:bg-neutral-950 text-neutral-900 dark:text-neutral-100 transition-colors relative">
        <ThemeProvider>
          <WalletProvider>
            <NetworkSecurityBanner />
            <BackgroundPattern />
            <FloatingScrollbar />
            <div className="relative z-10 flex flex-col flex-1">{children}</div>
          </WalletProvider>
        </ThemeProvider>
      </body>
    </html>
  );
}
