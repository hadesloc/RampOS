"use client";

import { useState, useEffect, useCallback } from "react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { toast } from "sonner";
import { Shield, User, Bell, Key, Loader2, LogOut } from "lucide-react";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { useRouter } from "@/navigation";
import { useAuth } from "@/contexts/auth-context";
import {
  settingsApi,
  type UserProfile,
  type SecuritySettings,
  type NotificationPreferences,
  PortalApiError,
} from "@/lib/portal-api";
import { useTranslations, useFormatter } from "next-intl";

export default function SettingsPage() {
  const [saving, setSaving] = useState(false);
  const [loadingProfile, setLoadingProfile] = useState(true);
  const [loadingSecurity, setLoadingSecurity] = useState(true);
  const [loadingNotifications, setLoadingNotifications] = useState(true);
  const t = useTranslations('Portal.settings');
  const tCommon = useTranslations('Common');
  const tNav = useTranslations('Navigation');
  const format = useFormatter();

  // Profile state
  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [fullName, setFullName] = useState("");
  const [phone, setPhone] = useState("");

  // Security state
  const [security, setSecurity] = useState<SecuritySettings | null>(null);

  // Notification state
  const [notifications, setNotifications] =
    useState<NotificationPreferences | null>(null);

  const {
    user,
    isAuthenticated,
    isLoading: authLoading,
    logout,
  } = useAuth();
  const router = useRouter();

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch profile data
  const fetchProfile = useCallback(async () => {
    try {
      setLoadingProfile(true);
      const data = await settingsApi.getProfile();
      setProfile(data);
      setFullName(data.fullName || "");
      setPhone(data.phone || "");
    } catch (err) {
      if (err instanceof PortalApiError && err.status !== 401) {
        toast.error("Failed to load profile");
      }
    } finally {
      setLoadingProfile(false);
    }
  }, []);

  // Fetch security data
  const fetchSecurity = useCallback(async () => {
    try {
      setLoadingSecurity(true);
      const data = await settingsApi.getSecurity();
      setSecurity(data);
    } catch (err) {
      if (err instanceof PortalApiError && err.status !== 401) {
        toast.error("Failed to load security settings");
      }
    } finally {
      setLoadingSecurity(false);
    }
  }, []);

  // Fetch notification preferences
  const fetchNotifications = useCallback(async () => {
    try {
      setLoadingNotifications(true);
      const data = await settingsApi.getNotifications();
      setNotifications(data);
    } catch (err) {
      if (err instanceof PortalApiError && err.status !== 401) {
        toast.error("Failed to load notification preferences");
      }
    } finally {
      setLoadingNotifications(false);
    }
  }, []);

  // Load data on mount
  useEffect(() => {
    if (isAuthenticated) {
      fetchProfile();
      fetchSecurity();
      fetchNotifications();
    }
  }, [isAuthenticated, fetchProfile, fetchSecurity, fetchNotifications]);

  // Save profile
  const handleSaveProfile = async () => {
    setSaving(true);
    try {
      const result = await settingsApi.updateProfile({
        fullName: fullName || undefined,
        phone: phone || undefined,
      });
      setProfile(result.profile);
      toast.success(tCommon('success'));
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : tCommon('error');
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  // Save notification preferences
  const handleSaveNotifications = async () => {
    if (!notifications) return;
    setSaving(true);
    try {
      const result = await settingsApi.updateNotifications(notifications);
      setNotifications(result.preferences);
      toast.success(tCommon('success'));
    } catch (err) {
      const message =
        err instanceof PortalApiError
          ? err.message
          : tCommon('error');
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  const handleLogout = async () => {
    try {
      await logout();
    } catch {
      toast.error(tCommon('error'));
    }
  };

  return (
    <PageContainer>
      <PageHeader
        title={t('title')}
        description={t('description')}
        actions={
          <Button variant="destructive" onClick={handleLogout}>
            <LogOut className="h-4 w-4 mr-2" />
            {tNav('logout')}
          </Button>
        }
      />

      <Tabs defaultValue="profile" className="space-y-4">
        <TabsList>
          <TabsTrigger value="profile" className="gap-2">
            <User className="h-4 w-4" />
            {t('profile')}
          </TabsTrigger>
          <TabsTrigger value="security" className="gap-2">
            <Shield className="h-4 w-4" />
            {t('security')}
          </TabsTrigger>
          <TabsTrigger value="notifications" className="gap-2">
            <Bell className="h-4 w-4" />
            {t('notifications')}
          </TabsTrigger>
        </TabsList>

        {/* Profile Tab */}
        <TabsContent value="profile">
          <Card>
            <CardHeader>
              <CardTitle>{t('profile')}</CardTitle>
              <CardDescription>
                Update your account details and contact information.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              {loadingProfile ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : (
                <>
                  <div className="space-y-2">
                    <Label htmlFor="name">{t('full_name')}</Label>
                    <Input
                      id="name"
                      value={fullName}
                      onChange={(e) => setFullName(e.target.value)}
                      placeholder="Your name"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="email">Email Address</Label>
                    <Input
                      id="email"
                      type="email"
                      value={profile?.email || user?.email || ""}
                      readOnly
                      className="bg-muted"
                    />
                    <p className="text-xs text-muted-foreground">
                      Email cannot be changed. Contact support if you need to
                      update it.
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="phone">{t('phone')}</Label>
                    <Input
                      id="phone"
                      type="tel"
                      value={phone}
                      onChange={(e) => setPhone(e.target.value)}
                      placeholder="+84 ..."
                    />
                  </div>
                  <Button onClick={handleSaveProfile} disabled={saving} aria-label="Save profile changes">
                    {saving ? tCommon('loading') : tCommon('save')}
                  </Button>
                </>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security">
          <Card>
            <CardHeader>
              <CardTitle>{t('security')}</CardTitle>
              <CardDescription>
                Protect your account with additional security measures.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {loadingSecurity ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : (
                <>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label className="text-base">
                        {t('2fa')}
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        Secure your account with an additional layer of
                        security.
                      </p>
                    </div>
                    <Switch
                      checked={security?.twoFactorEnabled ?? false}
                      disabled
                    />
                  </div>

                  <div className="border-t pt-6">
                    <h4 className="font-medium mb-4">{t('passkeys')}</h4>
                    {security?.webauthnCredentials &&
                    security.webauthnCredentials.length > 0 ? (
                      security.webauthnCredentials.map((cred) => (
                        <div
                          key={cred.id}
                          className="flex items-center justify-between p-4 border rounded-lg bg-muted/50 mb-2"
                        >
                          <div className="flex items-center gap-3">
                            <Key className="h-5 w-5 text-primary" />
                            <div>
                              <p className="font-medium">{cred.name}</p>
                              <p className="text-sm text-muted-foreground">
                                Added{" "}
                                {format.dateTime(new Date(cred.createdAt), {
                                  month: "short",
                                  day: "numeric",
                                  year: "numeric",
                                })}
                              </p>
                            </div>
                          </div>
                          <Button
                            variant="ghost"
                            size="sm"
                            className="text-destructive"
                          >
                            {tCommon('delete')}
                          </Button>
                        </div>
                      ))
                    ) : (
                      <p className="text-sm text-muted-foreground mb-2">
                        No passkeys registered yet.
                      </p>
                    )}
                    <Button variant="outline" className="mt-4 gap-2">
                      Add a passkey
                    </Button>
                  </div>

                  {security?.lastPasswordChange && (
                    <div className="border-t pt-6">
                      <p className="text-sm text-muted-foreground">
                        Last password change:{" "}
                        {format.dateTime(new Date(security.lastPasswordChange), {
                          month: "short",
                          day: "numeric",
                          year: "numeric",
                        })}
                      </p>
                    </div>
                  )}
                </>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        {/* Notifications Tab */}
        <TabsContent value="notifications">
          <Card>
            <CardHeader>
              <CardTitle>{t('notifications')}</CardTitle>
              <CardDescription>
                Choose what updates you want to receive.
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-6">
              {loadingNotifications ? (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              ) : (
                <>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label className="text-base">{t('email_notif')}</Label>
                      <p className="text-sm text-muted-foreground">
                        Get notified about deposits, withdrawals and trades via
                        email.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.emailNotifications ?? true}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev
                            ? { ...prev, emailNotifications: checked }
                            : prev
                        )
                      }
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label className="text-base">SMS Notifications</Label>
                      <p className="text-sm text-muted-foreground">
                        Receive alerts about login attempts and security changes
                        via SMS.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.smsNotifications ?? false}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev
                            ? { ...prev, smsNotifications: checked }
                            : prev
                        )
                      }
                    />
                  </div>
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label className="text-base">{t('push_notif')}</Label>
                      <p className="text-sm text-muted-foreground">
                        Receive push notifications about account activity.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.pushNotifications ?? true}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev
                            ? { ...prev, pushNotifications: checked }
                            : prev
                        )
                      }
                    />
                  </div>
                  <Button
                    onClick={handleSaveNotifications}
                    disabled={saving}
                    aria-label="Save notification preferences"
                  >
                    {saving ? tCommon('loading') : tCommon('save')}
                  </Button>
                </>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}
