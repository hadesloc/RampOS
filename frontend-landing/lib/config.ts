export const config = {
  dashboardUrl: process.env.NEXT_PUBLIC_DASHBOARD_URL || 'http://localhost:3000',
}

export function getDashboardUrl(path: string = ''): string {
  const baseUrl = config.dashboardUrl.replace(/\/$/, '')
  const cleanPath = path.startsWith('/') ? path : `/${path}`
  return `${baseUrl}${cleanPath}`
}
