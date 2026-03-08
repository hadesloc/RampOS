import { getRequestConfig } from 'next-intl/server';
import { routing } from './routing';

export default getRequestConfig(async ({ requestLocale }) => {
  // Read locale from the URL pathname (set by next-intl middleware)
  let locale = await requestLocale;

  // Validate and fall back to default locale
  if (!locale || !routing.locales.includes(locale as 'en' | 'vi')) {
    locale = routing.defaultLocale;
  }

  return {
    locale,
    messages: (await import(`../../messages/${locale}.json`)).default
  };
});