import { Input } from "../../components/Input";
import { useI18n } from "../../i18n";

interface Props {
  busy: boolean;
  formId: string;
  newSkillName: string;
  setNewSkillName: (value: string) => void;
  newSkillDescription: string;
  setNewSkillDescription: (value: string) => void;
  createSkill: () => void | Promise<void>;
}

/** Creates one editable local Skill from the built-in template. */
export function NewSkillForm({
  busy,
  formId,
  newSkillName,
  setNewSkillName,
  newSkillDescription,
  setNewSkillDescription,
  createSkill,
}: Props) {
  const { t } = useI18n();
  return (
    <form
      id={formId}
      className="asb-form"
      aria-label={t("extensions.toolbar.newSkill")}
      onSubmit={(event) => {
        event.preventDefault();
        if (busy) return;
        void createSkill();
      }}
    >
      <label className="asb-field">
        <span>{t("extensions.newSkill.name")}</span>
        <Input
          required
          value={newSkillName}
          disabled={busy}
          onChange={(event) => setNewSkillName(event.target.value)}
        />
      </label>
      <label className="asb-field">
        <span>{t("extensions.newSkill.description")}</span>
        <Input
          required
          placeholder={t("extensions.newSkill.descriptionPlaceholder")}
          value={newSkillDescription}
          disabled={busy}
          onChange={(event) => setNewSkillDescription(event.target.value)}
        />
      </label>
    </form>
  );
}
