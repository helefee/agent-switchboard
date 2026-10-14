import { useI18n } from "../i18n";
import { UpdateIcon, UpdateSpinnerIcon } from "./icons";
import { Button } from "./Button";
import { Tooltip } from "./Tooltip";

/** Sidebar entry above the settings item, always mounted so a manual check is
 * reachable from every page. Neutral while idle (click runs a check), spinner
 * while checking, green while a release is known — clicking then leads to the
 * settings page, where the release link lives. Stateless by design: the check
 * result stays with App. */
export function UpdateButton({
  checking,
  latestVersion,
  onCheck,
  onOpen,
}: {
  checking: boolean;
  latestVersion: string | null;
  onCheck: () => void;
  onOpen: () => void;
}) {
  const { t } = useI18n();
  const available = !checking && latestVersion !== null;
  const label = available
    ? t("update.available", { version: latestVersion })
    : checking
      ? t("update.checking")
      : t("update.check");
  return (
    <Tooltip label={label} side="right">
      <Button
        variant="unstyled"
        className={`asb-updatebtn${available ? " is-available" : ""}${checking ? " is-checking" : ""}`}
        aria-label={label}
        aria-busy={checking || undefined}
        disabled={checking}
        onClick={available ? onOpen : onCheck}
      >
        {checking ? <UpdateSpinnerIcon /> : <UpdateIcon />}
        <span className="asb-updatebtn-label">
          {available ? t("update.button") : t("update.check")}
        </span>
        {available ? <span className="asb-updatebtn-dot" aria-hidden="true" /> : null}
      </Button>
    </Tooltip>
  );
}
