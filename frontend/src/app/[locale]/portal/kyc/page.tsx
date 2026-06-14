"use client";

import { useState, useEffect, useCallback } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { Clock, CheckCircle2, XCircle, Loader2, Upload, Camera, ShieldCheck, FileCheck2, UserCheck, ScanFace } from "lucide-react";
import { KYCStatus, kycApi } from "@/lib/portal-api";
import { useAuth } from "@/contexts/auth-context";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Progress } from "@/components/ui/progress";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { KYCProgress } from "@/components/portal/kyc-progress";
import { PageHeader } from "@/components/layout/page-header";
import { PageContainer } from "@/components/layout/page-container";
import { useRouter } from "@/navigation";
import { useTranslations } from "next-intl";
import { StatGrid, StatCard, Panel, SectionCard, EmptyState, ErrorState, StatusBadge, CardGridSkeleton } from "@/components/shared";

const kycSchema = z.object({
  firstName: z.string().min(2, "First name must be at least 2 characters"),
  lastName: z.string().min(2, "Last name must be at least 2 characters"),
  dob: z.string().refine((val) => !isNaN(Date.parse(val)), "Invalid date"),
  address: z.string().min(5, "Address must be at least 5 characters"),
  idDocumentType: z.enum(["PASSPORT", "DRIVERS_LICENSE", "NATIONAL_ID"]),
});

type KYCFormData = z.infer<typeof kycSchema>;

type UploadedDocs = {
  idFront: File | null;
  idBack: File | null;
  selfie: File | null;
};

const inputClassName = "border-white/[0.08] bg-[#09090B] text-foreground placeholder:text-muted-foreground/50 focus-visible:ring-[#00D4FF]/40";
const uploadBoxClassName = "rounded-xl border border-dashed border-white/[0.12] bg-[#09090B]/80 p-6 text-center transition-colors hover:border-[#00D4FF]/40 hover:bg-[#00D4FF]/[0.03]";
const reviewBoxClassName = "rounded-xl border border-white/[0.08] bg-[#09090B]/80 p-4";

export default function KYCPage() {
  const [step, setStep] = useState(1);
  const [progress, setProgress] = useState(25);
  const [kycStatus, setKycStatus] = useState<KYCStatus | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isLoadingStatus, setIsLoadingStatus] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [uploadedDocs, setUploadedDocs] = useState<UploadedDocs>({
    idFront: null,
    idBack: null,
    selfie: null,
  });
  const [uploadProgress, setUploadProgress] = useState<Record<string, boolean>>(
    {}
  );
  const t = useTranslations('Portal.kyc');
  const tCommon = useTranslations('Common');
  const tPortal = useTranslations('Portal.dashboard');

  const { user, isAuthenticated, isLoading: authLoading } = useAuth();
  const router = useRouter();

  const {
    register,
    handleSubmit,
    trigger,
    formState: { errors },
    watch,
    setValue,
  } = useForm<KYCFormData>({
    resolver: zodResolver(kycSchema),
    mode: "onChange",
  });

  const formData = watch();

  // Redirect if not authenticated
  useEffect(() => {
    if (!authLoading && !isAuthenticated) {
      router.push("/portal/login");
    }
  }, [authLoading, isAuthenticated, router]);

  // Fetch KYC status
  const fetchKycStatus = useCallback(async () => {
    try {
      const status = await kycApi.getStatus();
      setKycStatus(status);
    } catch {
      // Failed to fetch KYC status silently
    } finally {
      setIsLoadingStatus(false);
    }
  }, []);

  useEffect(() => {
    if (isAuthenticated) {
      fetchKycStatus();
    }
  }, [isAuthenticated, fetchKycStatus]);

  const nextStep = async () => {
    let fieldsToValidate: (keyof KYCFormData)[] = [];

    if (step === 1) {
      fieldsToValidate = ["firstName", "lastName", "dob", "address", "idDocumentType"];
    }

    const isValid = await trigger(fieldsToValidate);

    if (isValid || step > 1) {
      setStep((prev) => Math.min(prev + 1, 4));
      setProgress((prev) => Math.min(prev + 25, 100));
    }
  };

  const prevStep = () => {
    setStep((prev) => Math.max(prev - 1, 1));
    setProgress((prev) => Math.max(prev - 25, 25));
  };

  const handleFileUpload = async (
    type: "idFront" | "idBack" | "selfie",
    file: File
  ) => {
    setUploadedDocs((prev) => ({ ...prev, [type]: file }));
    setUploadProgress((prev) => ({ ...prev, [type]: true }));

    try {
      const docType =
        type === "idFront"
          ? "ID_FRONT"
          : type === "idBack"
          ? "ID_BACK"
          : "SELFIE";
      await kycApi.uploadDocument(docType, file);
    } catch {
      setError(`Failed to upload ${type}. Please try again.`);
      setUploadedDocs((prev) => ({ ...prev, [type]: null }));
    } finally {
      setUploadProgress((prev) => ({ ...prev, [type]: false }));
    }
  };

  const onSubmit = async (data: KYCFormData) => {
    setIsSubmitting(true);
    setError(null);

    try {
      await kycApi.submit({
        firstName: data.firstName,
        lastName: data.lastName,
        dateOfBirth: data.dob,
        address: data.address,
        idDocumentType: data.idDocumentType,
      });

      // Refresh status
      await fetchKycStatus();
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to submit KYC. Please try again."
      );
    } finally {
      setIsSubmitting(false);
    }
  };

  const statusSeverity =
    kycStatus?.status === "VERIFIED"
      ? "success"
      : kycStatus?.status === "REJECTED"
      ? "danger"
      : kycStatus?.status === "PENDING"
      ? "warning"
      : "neutral";

  const uploadedCount = [uploadedDocs.idFront, uploadedDocs.idBack, uploadedDocs.selfie].filter(Boolean).length;
  const requiredDocsReady = Boolean(uploadedDocs.idFront && uploadedDocs.selfie);

  const renderUploadState = (
    type: "idFront" | "idBack" | "selfie",
    icon: "upload" | "camera",
    label: string,
    helper?: string
  ) => {
    const uploadedFile = uploadedDocs[type];
    const isUploading = uploadProgress[type];
    const Icon = icon === "camera" ? Camera : Upload;

    if (uploadedFile) {
      return (
        <div className="flex items-center justify-center gap-2 text-[#00FF87]">
          <CheckCircle2 className="h-5 w-5" />
          <span className="truncate font-medium">{uploadedFile.name}</span>
        </div>
      );
    }

    if (isUploading) {
      return (
        <div className="flex items-center justify-center gap-2 text-[#00D4FF]">
          <Loader2 className="h-5 w-5 animate-spin" />
          <span>{tCommon('loading')}</span>
        </div>
      );
    }

    return (
      <label className="cursor-pointer text-muted-foreground transition-colors hover:text-[#00D4FF]">
        <Icon className="mx-auto mb-3 h-9 w-9" />
        <span className="block text-sm font-medium">{label}</span>
        {helper && <span className="mt-1 block text-xs text-muted-foreground/70">{helper}</span>}
        <input
          id={type === "idFront" ? "idFrontUpload" : type === "idBack" ? "idBackUpload" : "selfieUpload"}
          type="file"
          className="hidden"
          accept={type === "selfie" ? "image/*" : "image/*,.pdf"}
          capture={type === "selfie" ? "user" : undefined}
          onChange={(e) => {
            const file = e.target.files?.[0];
            if (file) handleFileUpload(type, file);
          }}
        />
      </label>
    );
  };

  if (authLoading || (isAuthenticated && isLoadingStatus)) {
    return (
      <PageContainer>
        <PageHeader title={t('title')} description={t('description')} />
        <div className="mx-auto max-w-5xl space-y-6">
          <CardGridSkeleton cards={3} />
          <Panel>
            <div className="flex items-center justify-center py-10 text-muted-foreground">
              <Loader2 className="mr-2 h-5 w-5 animate-spin text-[#00D4FF]" />
              {tCommon('loading')}
            </div>
          </Panel>
        </div>
      </PageContainer>
    );
  }

  // Show KYC status if already submitted
  if (kycStatus && kycStatus.status !== "NONE") {
    return (
      <PageContainer>
        <PageHeader title={t('title')} description={t('description')} />
        <div className="mx-auto max-w-5xl space-y-6">
          <StatGrid cols={3}>
            <StatCard title={tCommon('status')} value={kycStatus.status} icon={<ShieldCheck className="h-4 w-4" />} accentColor={statusSeverity === "danger" ? "amber" : statusSeverity === "success" ? "green" : "violet"} subtitle={user?.email || t('description')} />
            <StatCard title={t('step_4')} value="4 / 4" icon={<FileCheck2 className="h-4 w-4" />} accentColor="cyan" subtitle={t('step_4_desc')} />
            <StatCard title="KYC Tier" value={kycStatus.tier ? `Level ${kycStatus.tier}` : "—"} icon={<UserCheck className="h-4 w-4" />} accentColor="amber" subtitle={kycStatus.submittedAt ? `Submitted ${new Date(kycStatus.submittedAt).toLocaleDateString()}` : t('description')} />
          </StatGrid>

          <SectionCard
            header={{
              title: t('title'),
              description: t('description'),
              actions: <StatusBadge status={kycStatus.status} severity={statusSeverity} />,
            }}
          >
            <KYCProgress
              currentStep={4}
              steps={[
                { label: t('step_1'), completed: true },
                { label: t('step_2'), completed: true },
                { label: t('step_3'), completed: true },
                { label: t('step_4'), completed: true }
              ]}
              status={kycStatus.status}
            />

            {kycStatus.status === "PENDING" && (
              <div className="flex flex-col items-center space-y-4 py-8 text-center">
                <div className="rounded-full border border-[#FFB800]/20 bg-[#FFB800]/10 p-4 text-[#FFB800] shadow-[0_0_32px_rgba(255,184,0,0.12)]">
                  <Clock className="h-12 w-12" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold text-foreground">{t('pending')}</h2>
                  <p className="max-w-md text-muted-foreground">
                    {tPortal('kyc_pending')}
                  </p>
                </div>
                {kycStatus.submittedAt && (
                  <p className="text-sm text-muted-foreground">
                    Submitted on {new Date(kycStatus.submittedAt).toLocaleDateString()}
                  </p>
                )}
              </div>
            )}

            {kycStatus.status === "VERIFIED" && (
              <div className="flex flex-col items-center space-y-4 py-8 text-center">
                <div className="rounded-full border border-[#00FF87]/20 bg-[#00FF87]/10 p-4 text-[#00FF87] shadow-[0_0_32px_rgba(0,255,135,0.12)]">
                  <CheckCircle2 className="h-12 w-12" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold text-foreground">{t('verified')}</h2>
                  <p className="max-w-md text-muted-foreground">
                    Your identity has been verified. You now have full access to
                    all platform features.
                  </p>
                </div>
                <div className="rounded-xl border border-[#00FF87]/20 bg-[#00FF87]/10 px-4 py-3">
                  <p className="text-sm">
                    <span className="text-muted-foreground">KYC Tier:</span>{" "}
                    <span className="font-medium text-[#00FF87]">Level {kycStatus.tier}</span>
                  </p>
                </div>
              </div>
            )}

            {kycStatus.status === "REJECTED" && (
              <div className="flex flex-col items-center space-y-4 py-8 text-center">
                <div className="rounded-full border border-red-400/20 bg-red-400/10 p-4 text-red-400 shadow-[0_0_32px_rgba(248,113,113,0.12)]">
                  <XCircle className="h-12 w-12" />
                </div>
                <div className="space-y-2">
                  <h2 className="text-xl font-semibold text-foreground">{t('failed')}</h2>
                  <p className="max-w-md text-muted-foreground">
                    {tPortal('kyc_rejected')}
                  </p>
                </div>
                {kycStatus.rejectionReason && (
                  <ErrorState title={t('failed')} message={kycStatus.rejectionReason} className="py-6" />
                )}
                <Button
                  onClick={() => {
                    setKycStatus({ ...kycStatus, status: "NONE" });
                    setStep(1);
                    setProgress(25);
                  }}
                  className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90"
                >
                  {tCommon('try_again')}
                </Button>
              </div>
            )}
          </SectionCard>
        </div>
      </PageContainer>
    );
  }

  const steps = [
    { label: t('step_1'), completed: step > 1 },
    { label: t('step_2'), completed: step > 2 },
    { label: t('step_3'), completed: step > 3 },
    { label: t('step_4'), completed: step > 4 }
  ];

  return (
    <PageContainer>
      <PageHeader title={t('title')} description={t('description')} />

      <div className="mx-auto max-w-5xl space-y-6">
        <StatGrid cols={3}>
          <StatCard title={tCommon('status')} value="Incomplete" icon={<ShieldCheck className="h-4 w-4" />} accentColor="violet" subtitle={user?.email || t('description')} />
          <StatCard title="Current step" value={`${step} / 4`} icon={<ScanFace className="h-4 w-4" />} accentColor="cyan" subtitle={steps[step - 1]?.label} />
          <StatCard title="Documents" value={`${uploadedCount} / 3`} icon={<FileCheck2 className="h-4 w-4" />} accentColor={requiredDocsReady ? "green" : "amber"} subtitle={requiredDocsReady ? tCommon('success') : t('step_2')} />
        </StatGrid>

        <Panel>
          <div className="space-y-5">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div>
                <p className="text-sm font-semibold text-foreground">{t('title')}</p>
                <p className="text-xs text-muted-foreground">{t('description')}</p>
              </div>
              <StatusBadge status="Not submitted" severity="neutral" />
            </div>
            <KYCProgress
              currentStep={step}
              steps={steps}
              status={kycStatus?.status || 'NONE'}
            />
            <div className="space-y-2">
              <div className="flex items-center justify-between text-xs text-muted-foreground">
                <span>{steps[step - 1]?.label}</span>
                <span>{progress}%</span>
              </div>
              <Progress value={progress} className="h-2 bg-white/[0.06]" />
            </div>
          </div>
        </Panel>

        {error && (
          <Panel>
            <ErrorState title={tCommon('error')} message={error} className="py-6" />
          </Panel>
        )}

        <Panel
          header={{
            title: steps[step - 1]?.label,
            description: step === 1 ? t('step_1_desc') : step === 2 ? `Upload clear pictures of your ${formData.idDocumentType?.toLowerCase().replace("_", " ") || "ID"}.` : step === 3 ? t('step_3_desc') : t('step_4_desc'),
            actions: <StatusBadge status={`Step ${step}`} severity="info" />,
          }}
          footer={
            <div className="flex w-full justify-between gap-3">
              {step > 1 ? (
                <Button type="button" variant="outline" onClick={prevStep} className="border-white/[0.1] bg-transparent hover:border-[#7B61FF]/50 hover:bg-[#7B61FF]/10">
                  {tCommon('back')}
                </Button>
              ) : (
                <div />
              )}

              {step < 4 ? (
                <Button type="button" onClick={nextStep} className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90">
                  {tCommon('next')}
                </Button>
              ) : (
                <Button
                  type="submit"
                  form="kyc-form"
                  disabled={
                    isSubmitting || !uploadedDocs.idFront || !uploadedDocs.selfie
                  }
                  className="bg-[#00FF87] text-black hover:bg-[#00FF87]/90 disabled:bg-white/10 disabled:text-muted-foreground"
                >
                  {isSubmitting && (
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  )}
                  {tCommon('submit')}
                </Button>
              )}
            </div>
          }
        >
          <form id="kyc-form" onSubmit={handleSubmit(onSubmit)}>
            {step === 1 && (
              <div className="space-y-4">
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label htmlFor="firstName">{t('first_name')}</Label>
                    <Input
                      id="firstName"
                      className={inputClassName}
                      {...register("firstName")}
                      placeholder="John"
                    />
                    {errors.firstName && (
                      <p className="text-sm text-red-400">
                        {errors.firstName?.message as string}
                      </p>
                    )}
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="lastName">{t('last_name')}</Label>
                    <Input
                      id="lastName"
                      className={inputClassName}
                      {...register("lastName")}
                      placeholder="Doe"
                    />
                    {errors.lastName && (
                      <p className="text-sm text-red-400">
                        {errors.lastName?.message as string}
                      </p>
                    )}
                  </div>
                </div>
                <div className="space-y-2">
                  <Label htmlFor="dob">{t('dob')}</Label>
                  <Input id="dob" type="date" className={inputClassName} {...register("dob")} />
                  {errors.dob && (
                    <p className="text-sm text-red-400">
                      {errors.dob?.message as string}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="address">{t('address')}</Label>
                  <Input
                    id="address"
                    className={inputClassName}
                    {...register("address")}
                    placeholder="123 Main St, City, Country"
                  />
                  {errors.address && (
                    <p className="text-sm text-red-400">
                      {errors.address?.message as string}
                    </p>
                  )}
                </div>
                <div className="space-y-2">
                  <Label htmlFor="idDocumentType">{t('id_type')}</Label>
                  <Select
                    onValueChange={(value) =>
                      setValue(
                        "idDocumentType",
                        value as "PASSPORT" | "DRIVERS_LICENSE" | "NATIONAL_ID"
                      )
                    }
                    defaultValue={formData.idDocumentType}
                  >
                    <SelectTrigger className={inputClassName}>
                      <SelectValue placeholder="Select document type" />
                    </SelectTrigger>
                    <SelectContent className="border-white/[0.08] bg-[#111113]">
                      <SelectItem value="PASSPORT">Passport</SelectItem>
                      <SelectItem value="DRIVERS_LICENSE">
                        Driver&apos;s License
                      </SelectItem>
                      <SelectItem value="NATIONAL_ID">National ID Card</SelectItem>
                    </SelectContent>
                  </Select>
                  {errors.idDocumentType && (
                    <p className="text-sm text-red-400">
                      {errors.idDocumentType?.message as string}
                    </p>
                  )}
                </div>
              </div>
            )}

            {step === 2 && (
              <div className="space-y-6">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label>{t('id_front')}</Label>
                    <div className={uploadBoxClassName}>
                      {renderUploadState("idFront", "upload", t('upload_front'))}
                    </div>
                  </div>

                  <div className="space-y-2">
                    <Label>{t('id_back')}</Label>
                    <div className={uploadBoxClassName}>
                      {renderUploadState("idBack", "upload", t('upload_back'))}
                    </div>
                  </div>
                </div>
                <p className="rounded-lg border border-[#00D4FF]/20 bg-[#00D4FF]/10 px-3 py-2 text-sm text-[#00D4FF]">
                  Supported formats: JPG, PNG, PDF. Max file size: 10MB
                </p>
              </div>
            )}

            {step === 3 && (
              <div className="space-y-4">
                <div className={uploadBoxClassName}>
                  {renderUploadState("selfie", "camera", t('upload_selfie'), "Make sure your face is clearly visible")}
                </div>
              </div>
            )}

            {step === 4 && (
              <div className="space-y-4">
                <div className={reviewBoxClassName}>
                  <div className="grid grid-cols-3 gap-3 text-sm">
                    <span className="font-medium text-muted-foreground">
                      {t('first_name')}:
                    </span>
                    <span className="col-span-2 text-foreground">
                      {formData.firstName} {formData.lastName}
                    </span>

                    <span className="font-medium text-muted-foreground">
                      {t('dob')}:
                    </span>
                    <span className="col-span-2 text-foreground">{formData.dob}</span>

                    <span className="font-medium text-muted-foreground">
                      {t('address')}:
                    </span>
                    <span className="col-span-2 text-foreground">{formData.address}</span>

                    <span className="font-medium text-muted-foreground">
                      {t('id_type')}:
                    </span>
                    <span className="col-span-2 text-foreground">
                      {formData.idDocumentType?.replace("_", " ")}
                    </span>
                  </div>
                </div>

                <div className={reviewBoxClassName}>
                  <p className="mb-3 text-sm font-medium text-foreground">Uploaded Documents</p>
                  <div className="space-y-2 text-sm">
                    <div className="flex items-center gap-2">
                      {uploadedDocs.idFront ? (
                        <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-400" />
                      )}
                      <span>{t('id_front')}</span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.idBack ? (
                        <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                      ) : (
                        <span className="h-4 w-4" />
                      )}
                      <span className="text-muted-foreground">
                        {t('id_back')} (optional)
                      </span>
                    </div>
                    <div className="flex items-center gap-2">
                      {uploadedDocs.selfie ? (
                        <CheckCircle2 className="h-4 w-4 text-[#00FF87]" />
                      ) : (
                        <XCircle className="h-4 w-4 text-red-400" />
                      )}
                      <span>{t('step_3')}</span>
                    </div>
                  </div>
                </div>

                {!requiredDocsReady && (
                  <EmptyState
                    icon={<FileCheck2 className="h-10 w-10" />}
                    title="Required documents are incomplete"
                    description="Upload the front of your ID and a selfie before submitting."
                    className="rounded-xl border border-[#FFB800]/20 bg-[#FFB800]/10 py-8"
                  />
                )}

                <div className="rounded-lg border border-white/[0.06] bg-white/[0.02] px-3 py-2 text-sm text-muted-foreground">
                  By submitting, you agree to our Terms of Service and Privacy
                  Policy.
                </div>
              </div>
            )}
          </form>
        </Panel>
      </div>
    </PageContainer>
  );
}
