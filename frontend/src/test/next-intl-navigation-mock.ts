export function createSharedPathnamesNavigation(_config: any) {
  return {
    Link: ({ children, href, ...props }: any) => {
      const React = require('react')
      return React.createElement('a', { href, ...props }, children)
    },
    redirect: () => {},
    usePathname: () => '/',
    useRouter: () => ({
      push: () => {},
      replace: () => {},
      prefetch: () => {},
      back: () => {},
      forward: () => {},
    }),
  }
}
