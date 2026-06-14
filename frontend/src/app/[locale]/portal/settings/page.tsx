"use client";

import { useState, useEffect, useCallback } from "react";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Switch } from "@/components/ui/switch";
import { toast } from "sonner";
import {
  Shield,
  User,
  Bell,
  Key,
  Loader2,
  LogOut,
  AlertCircle,
  Lock,
} from "lucide-react";
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
  const t = useTranslations("Portal.settings");
  const tCommon = useTranslations("Common");
  const tNav = useTranslations("Navigation");
  const format = useFormatter();

  const [profile, setProfile] = useState<UserProfile | null>(null);
  const [fullName, setFullName] = useState("");
  const [phone, setPhone] = useState("");
  const [security, setSecurity] = useState<SecuritySettings | null>(null);
  const [notifications, setNotifications] =
    useState<NotificationPreferences | null>(null);

  const {
    user,
    isAuthenticated,
    isLoading: authLoading,
    logout,
  } = useAuth();
  const router = useRouter();
  const passkeyUnavailableMessage =
    "Passkey management is unavailable because portal passkey completion and management APIs are not enabled in this environment.";

  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

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

  useEffect(() => {
    if (isAuthenticated) {
      fetchProfile();
      fetchSecurity();
      fetchNotifications();
    }
  }, [isAuthenticated, fetchProfile, fetchSecurity, fetchNotifications]);

  const handleSaveProfile = async () => {
    setSaving(true);
    try {
      const result = await settingsApi.updateProfile({
        fullName: fullName || undefined,
        phone: phone || undefined,
      });
      setProfile(result.profile);
      toast.success(tCommon("success"));
    } catch (err) {
      const message =
        err instanceof PortalApiError ? err.message : tCommon("error");
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  const handleSaveNotifications = async () => {
    if (!notifications) return;
    setSaving(true);
    try {
      const result = await settingsApi.updateNotifications(notifications);
      setNotifications(result.preferences);
      toast.success(tCommon("success"));
    } catch (err) {
      const message =
        err instanceof PortalApiError ? err.message : tCommon("error");
      toast.error(message);
    } finally {
      setSaving(false);
    }
  };

  const handleLogout = async () => {
    try {
      await logout();
    } catch {
      toast.error(tCommon("error"));
    }
  };

  return (
    <PageContainer>
      <PageHeader
        title={t("title")}
        description={t("description")}
        actions={
          <Button
            variant="destructive"
            onClick={handleLogout}
            className="gap-2"
          >
            <LogOut className="h-4 w-4" />
            {tNav("logout")}
          </Button>
        }
      />

      <Tabs defaultValue="profile" className="space-y-6">
        <TabsList className="bg-[#111113] border border-white/[0.06] p-1 h-auto gap-1">
          <TabsTrigger
            value="profile"
            className="gap-2 data-[state=active]:bg-[#00FF87]/10 data-[state=active]:text-[#00FF87] text-white/50"
          >
            <User className="h-4 w-4" />
            {t("profile")}
          </TabsTrigger>
          <TabsTrigger
            value="security"
            className="gap-2 data-[state=active]:bg-[#7B61FF]/10 data-[state=active]:text-[#7B61FF] text-white/50"
          >
            <Shield className="h-4 w-4" />
            {t("security")}
          </TabsTrigger>
          <TabsTrigger
            value="notifications"
            className="gap-2 data-[state=active]:bg-[#00D4FF]/10 data-[state=active]:text-[#00D4FF] text-white/50"
          >
            <Bell className="h-4 w-4" />
            {t("notifications")}
          </TabsTrigger>
        </TabsList>

        {/* Profile Tab */}
        <TabsContent value="profile">
          <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
            <div className="px-6 py-4 border-b border-white/[0.06]">
              <h3 className="font-semibold text-white">{t("profile")}</h3>
              <p className="text-sm text-white/40 mt-0.5">
                Update your account details and contact information.
              </p>
            </div>
            <div className="p-6">
              {loadingProfile ? (
                <div className="flex items-center justify-center py-10">
                  <Loader2 className="h-6 w-6 animate-spin text-[#00FF87]" />
                </div>
              ) : (
                <div className="space-y-5">
                  <div className="space-y-2">
                    <Label htmlFor="name" className="text-white/70 text-sm">
                      {t("full_name")}
                    </Label>
                    <Input
                      id="name"
                      value={fullName}
                      onChange={(e) => setFullName(e.target.value)}
                      placeholder="Your name"
                      className="bg-[#09090B] border-white/[0.08] text-white placeholder:text-white/20 focus-visible:border-[#00FF87]/40 focus-visible:ring-[#00FF87]/10"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="email" className="text-white/70 text-sm">
                      Email Address
                    </Label>
                    <Input
                      id="email"
                      type="email"
                      value={profile?.email || user?.email || ""}
                      readOnly
                      className="bg-[#09090B]/60 border-white/[0.06] text-white/40 cursor-not-allowed"
                    />
                    <p className="text-xs text-white/25">
                      Email cannot be changed. Contact support if you need to update it.
                    </p>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="phone" className="text-white/70 text-sm">
                      {t("phone")}
                    </Label>
                    <Input
                      id="phone"
                      type="tel"
                      value={phone}
                      onChange={(e) => setPhone(e.target.value)}
                      placeholder="+84 ..."
                      className="bg-[#09090B] border-white/[0.08] text-white placeholder:text-white/20 focus-visible:border-[#00FF87]/40 focus-visible:ring-[#00FF87]/10"
                    />
                  </div>
                  <Button
                    onClick={handleSaveProfile}
                    disabled={saving}
                    aria-label="Save profile changes"
                    className="bg-[#00FF87] text-[#09090B] font-semibold hover:bg-[#00FF87]/90 disabled:opacity-50"
                  >
                    {saving ? (
                      <>
                        <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                        {tCommon("loading")}
                      </>
                    ) : (
                      tCommon("save")
                    )}
                  </Button>
                </div>
              )}
            </div>
          </div>
        </TabsContent>

        {/* Security Tab */}
        <TabsContent value="security">
          <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
            <div className="px-6 py-4 border-b border-white/[0.06]">
              <h3 className="font-semibold text-white">{t("security")}</h3>
              <p className="text-sm text-white/40 mt-0.5">
                Protect your account with additional security measures.
              </p>
            </div>
            <div className="p-6">
              {loadingSecurity ? (
                <div className="flex items-center justify-center py-10">
                  <Loader2 className="h-6 w-6 animate-spin text-[#7B61FF]" />
                </div>
              ) : (
                <div className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <p className="text-sm font-medium text-white">
                        {t("2fa")}
                      </p>
                      <p className="text-sm text-white/40">
                        Secure your account with an additional layer of security.
                      </p>
                    </div>
                    <Switch
                      checked={security?.twoFactorEnabled ?? false}
                      disabled
                      className="data-[state=checked]:bg-[#7B61FF]"
                    />
                  </div>

                  <div className="border-t border-white/[0.06] pt-6">
                    <h4 className="text-sm font-medium text-white mb-4">
                      {t("passkeys")}
                    </h4>
                    <div className="flex items-start gap-3 rounded-lg border border-[#FFB800]/20 bg-[#FFB800]/[0.06] p-4">
                      <Lock className="h-4 w-4 mt-0.5 shrink-0 text-[#FFB800]" />
                      <p className="text-sm text-[#FFB800]/80">
                        {passkeyUnavailableMessage}
                      </p>
                    </div>
                    {security?.webauthnCredentials &&
                      security.webauthnCredentials.length > 0 && (
                        <div className="mt-4 space-y-2">
                          {security.webauthnCredentials.map((cred) => (
                            <div
                              key={cred.id}
                              className="flex items-center gap-3 rounded-lg border border-white/[0.06] bg-[#09090B]/60 p-4"
                            >
                              <Key className="h-5 w-5 text-[#7B61FF]" />
                              <div>
                                <p className="text-sm font-medium text-white">
                                  {cred.name}
                                </p>
                                <p className="text-xs text-white/40">
                                  Added{" "}
                                  {format.dateTime(new Date(cred.createdAt), {
                                    month: "short",
                                    day: "numeric",
                                    year: "numeric",
                                  })}
                                </p>
                              </div>
                            </div>
                          ))}
                        </div>
                      )}
                  </div>

                  {security?.lastPasswordChange && (
                    <div className="border-t border-white/[0.06] pt-6">
                      <p className="text-sm text-white/40">
                        Last password change:{" "}
                        {format.dateTime(
                          new Date(security.lastPasswordChange),
                          {
                            month: "short",
                            day: "numeric",
                            year: "numeric",
                          }
                        )}
                      </p>
                    </div>
                  )}
                </div>
              )}
            </div>
          </div>
        </TabsContent>

        {/* Notifications Tab */}
        <TabsContent value="notifications">
          <div className="rounded-xl border border-white/[0.06] bg-[#111113] overflow-hidden">
            <div className="px-6 py-4 border-b border-white/[0.06]">
              <h3 className="font-semibold text-white">{t("notifications")}</h3>
              <p className="text-sm text-white/40 mt-0.5">
                Choose what updates you want to receive.
              </p>
            </div>
            <div className="p-6">
              {loadingNotifications ? (
                <div className="flex items-center justify-center py-10">
                  <Loader2 className="h-6 w-6 animate-spin text-[#00D4FF]" />
                </div>
              ) : (
                <div className="space-y-6">
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <p className="text-sm font-medium text-white">
                        {t("email_notif")}
                      </p>
                      <p className="text-sm text-white/40">
                        Get notified about deposits, withdrawals and trades via email.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.emailNotifications ?? true}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev ? { ...prev, emailNotifications: checked } : prev
                        )
                      }
                      className="data-[state=checked]:bg-[#00D4FF]"
                    />
                  </div>

                  <div className="border-t border-white/[0.06] pt-6 flex items-center justify-between">
                    <div className="space-y-0.5">
                      <p className="text-sm font-medium text-white">
                        SMS Notifications
                      </p>
                      <p className="text-sm text-white/40">
                        Receive alerts about login attempts and security changes via SMS.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.smsNotifications ?? false}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev ? { ...prev, smsNotifications: checked } : prev
                        )
                      }
                      className="data-[state=checked]:bg-[#00D4FF]"
                    />
                  </div>

                  <div className="border-t border-white/[0.06] pt-6 flex items-center justify-between">
                    <div className="space-y-0.5">
                      <p className="text-sm font-medium text-white">
                        {t("push_notif")}
                      </p>
                      <p className="text-sm text-white/40">
                        Receive push notifications about account activity.
                      </p>
                    </div>
                    <Switch
                      checked={notifications?.pushNotifications ?? true}
                      onCheckedChange={(checked) =>
                        setNotifications((prev) =>
                          prev ? { ...prev, pushNotifications: checked } : prev
                        )
                      }
                      className="data-[state=checked]:bg-[#00D4FF]"
                    />
                  </div>

                  <div className="border-t border-white/[0.06] pt-6">
                    <Button
                      onClick={handleSaveNotifications}
                      disabled={saving}
                      aria-label="Save notification preferences"
                      className="bg-[#00D4FF] text-[#09090B] font-semibold hover:bg-[#00D4FF]/90 disabled:opacity-50"
                    >
                      {saving ? (
                        <>
                          <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                          {tCommon("loading")}
                        </>
                      ) : (
                        tCommon("save")
                      )}
                    </Button>
                  </div>
                </div>
              )}
            </div>
          </div>
        </TabsContent>
      </Tabs>
    </PageContainer>
  );
}
