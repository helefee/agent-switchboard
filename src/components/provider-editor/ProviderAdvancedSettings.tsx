import type { ReactNode } from "react";
import { useI18n } from "../../i18n";

interface Props {
  children: ReactNode;
}

export function ProviderAdvancedSettings({ children }: Props) {
  const { t } = useI18n();
  return (
    <section className="asb-editor-section" aria-label={t("providers.editor.section.advanced")}>
      <h3 className="asb-section-title">{t("providers.editor.section.advanced")}</h3>
      <div className="asb-editor-section-fields asb-provider-advanced-content">{children}</div>
    </section>
  );
}
