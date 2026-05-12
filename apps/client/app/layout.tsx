import type { Metadata, Viewport } from "next";
import { Geist, Geist_Mono, Instrument_Serif } from "next/font/google";
import { Analytics } from "@vercel/analytics/next";

import { SolanaWalletProvider } from "@/components/wallet-provider";
import { ToastProvider } from "@/components/toast";
import "./globals.css";

const geistSans = Geist({
  variable: "--font-geist-sans",
  subsets: ["latin"],
});

const geistMono = Geist_Mono({
  variable: "--font-geist-mono",
  subsets: ["latin"],
});

// High-contrast transitional serif used on the marketing surface only.
// The dashboard product UI stays sans-serif; serif is reserved for the
// editorial hero / feature copy on /, /docs hero, and similar pages.
const instrumentSerif = Instrument_Serif({
  variable: "--font-serif",
  weight: ["400"],
  style: ["normal", "italic"],
  subsets: ["latin"],
});

export const metadata: Metadata = {
  title: "Nyxbid",
  description: "Sealed-bid OTC for Solana.",
  icons: {
    icon: [{ url: "/nyxlogo.png", type: "image/png" }],
    apple: "/nyxlogo.png",
  },
};

export const viewport: Viewport = {
  themeColor: "#0a0a0d",
  colorScheme: "dark",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html
      lang="en"
      className={`${geistSans.variable} ${geistMono.variable} ${instrumentSerif.variable} h-full antialiased`}
    >
      <body className="min-h-screen">
        <SolanaWalletProvider>
          <ToastProvider>{children}</ToastProvider>
        </SolanaWalletProvider>
        <Analytics />
      </body>
    </html>
  );
}
