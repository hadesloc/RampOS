"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { ReactQueryDevtools } from "@tanstack/react-query-devtools";
import { Provider as UrqlProvider } from "urql";
import { ThemeProvider } from "next-themes";
import { useState } from "react";
import { graphqlClient } from "@/lib/graphql-client";
import { installCsrfProxyInterceptor } from "@/lib/csrf-interceptor";

// Install before any child component mounts so the first proxy fetch already
// carries a CSRF token (effect order runs children before parents).
installCsrfProxyInterceptor();

export default function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());

  return (
    <UrqlProvider value={graphqlClient}>
      <QueryClientProvider client={queryClient}>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false} forcedTheme="dark">
          {children}
        </ThemeProvider>
        <ReactQueryDevtools initialIsOpen={false} />
      </QueryClientProvider>
    </UrqlProvider>
  );
}
