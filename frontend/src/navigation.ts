import {createNavigation} from 'next-intl/navigation';
import {routing} from './i18n/routing';

export const locales = ['en', 'vi'] as const;
export const localePrefix = 'always'; // Default

export const {Link, redirect, usePathname, useRouter} =
  createNavigation(routing);
