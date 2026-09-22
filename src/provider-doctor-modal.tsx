import { CheckCircle2, Info, RefreshCw, ShieldAlert } from "lucide-react";
import { Badge as UiBadge } from "./components/ui/badge";
import { Button } from "./components/ui/button";
import type { ProviderDoctorResult } from "./backend-types";
import { t } from "./i18n";
import { providerDoctorSteps } from "./provider-doctor-steps";
import { isSuccessStatus } from "./status-presentation";

export function ProviderDoctorModal({ result, running, onClose }: {
  result: ProviderDoctorResult | null;
  running: boolean;
  onClose: () => void;
}) {
  const steps = providerDoctorSteps(result, running);
  const doneCount = steps.filter((step) => step.state === "ok" || step.state === "warning" || step.state === "failed").length;
  const progress = Math.round((doneCount / steps.length) * 100);
  return (
    <div className="modal-backdrop" role="dialog" aria-modal="true">
      <div className="modal-card provider-doctor-modal">
        <div className="modal-head">
          <div>
            <h2>Provider Doctor</h2>
            <p>{running ? t("正在诊断供应商，请稍候。") : result?.summary ?? t("诊断已完成。")}</p>
          </div>
          <UiBadge variant={result && !isSuccessStatus(result.status) ? "outline" : "secondary"}>
            {running ? t("诊断中") : result && !isSuccessStatus(result.status) ? t("异常") : t("完成")}
          </UiBadge>
        </div>
        <div className="provider-doctor-progress" aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress} role="progressbar">
          <div style={{ width: `${progress}%` }} />
        </div>
        <div className="provider-doctor-step-list">
          {steps.map((step) => (
            <div className={`provider-doctor-step ${step.state}`} key={step.id}>
              <span className="provider-doctor-step-icon">
                {step.state === "running" ? (
                  <RefreshCw className="h-4 w-4" />
                ) : step.state === "ok" ? (
                  <CheckCircle2 className="h-4 w-4" />
                ) : step.state === "warning" ? (
                  <ShieldAlert className="h-4 w-4" />
                ) : step.state === "failed" ? (
                  <Info className="h-4 w-4" />
                ) : (
                  <span />
                )}
              </span>
              <div>
                <strong>{step.title}</strong>
                <small>{step.detail}</small>
              </div>
            </div>
          ))}
        </div>
        {result?.recommendation ? <p className="provider-doctor-recommendation">{result.recommendation}</p> : null}
        <div className="modal-actions">
          <Button disabled={running} onClick={onClose} variant="secondary">
            {running ? t("诊断中") : t("关闭")}
          </Button>
        </div>
      </div>
    </div>
  );
}
