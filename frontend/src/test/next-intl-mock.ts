export function useTranslations(_namespace?: string) {
  return (key: string) => key
}

export function useLocale() {
  return 'en'
}

export function useMessages() {
  return {}
}

export function NextIntlClientProvider({ children }: any) {
  return children
}
