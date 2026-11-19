import type {
  CaptureSource,
  HistorySettingsSnapshot,
  InferenceBackendMetadata,
  InferenceCredentialReadinessSnapshot,
  ModelSettingsSnapshot,
  ProcessingConversionSettings,
  ProcessingProfileDraft_Serialize,
  ProviderSnapshot,
} from '$lib/generated/bindings';

export type Preview = {
  src: string;
  mimeType: 'image/png' | 'image/jpeg';
  width: number;
  height: number;
};

/** The complete editable payload for one Profile. */
export type ProcessingProfileDraft = Omit<
  ProcessingProfileDraft_Serialize,
  'conversion' | 'modelSettings' | 'rulesModelSettings'
> & {
  conversion: ProcessingConversionSettings;
  modelSettings: ModelSettingsDraft | null;
  rulesModelSettings: ModelSettingsDraft | null;
};

export type InferenceProviderDraft = Omit<ProviderSnapshot, 'id' | 'credentialRef'> & {
  credentialRef: string | null;
};

export type ModelSettingsDraft = Pick<ModelSettingsSnapshot, 'provider' | 'model' | 'options'>;

export type InferenceProviderCredentialDraft = Omit<InferenceProviderDraft, 'credentialRef'> & {
  credential: string;
};

export type InferenceProfilesViewState = {
  metadata: InferenceBackendMetadata[];
  metadataLoaded: boolean;
  pending:
    | 'create'
    | 'update'
    | 'rename'
    | 'delete'
    | 'assign'
    | 'credential'
    | 'provider-create'
    | 'provider-update'
    | 'provider-delete'
    | 'provider-credential'
    | null;
  readiness: Record<string, InferenceCredentialReadinessSnapshot>;
  error: string | null;
};

export type ProcessingPromptTemplateDefaults =
  import('$lib/generated/bindings').PromptTemplateDefaults;

export type ProcessingProfilesViewState = {
  pending: 'create' | 'update' | 'rename' | 'delete' | 'select' | null;
  error: string | null;
};

export type CaptureViewState = {
  busy: boolean;
  activeSource: CaptureSource | null;
  phase: CapturePhase;
  downloadProgress?: number | null;
  errorMessage: string | null;
  noticeMessage: string | null;
  preview: Preview | null;
  htmlResult: string | null;
  htmlResultAnnouncement: string;
  statusMessage: string;
};

export type CapturePhase =
  'idle' | 'selecting' | 'processing' | 'loading' | 'downloading' | 'failed';

export type ShortcutViewState = {
  newBindingProfileId: string;
  newBindingShortcut: string;
  recordingTarget: ShortcutRecordingTarget | null;
  recording: boolean;
  recordingPending: boolean;
  saving: boolean;
  error: string | null;
};

export type ShortcutRecordingTarget =
  { kind: 'binding'; bindingId: string } | { kind: 'create'; profileId: string } | { kind: 'open' };

export type DesktopAvailability = {
  desktopAvailable: boolean;
};

export type HistoryEntry = {
  id: string;
  /** Storage order of a saved entry; null while the capture is still pending. */
  sequence: number | null;
  createdAt: string;
  thumbnail: string | null;
  /** Whether the full-resolution capture is still kept for this entry. */
  hasOriginal: boolean;
  captureSource: string;
  processingProfile: string;
  settingsSnapshot: HistorySettingsSnapshot | null;
  imageWidth: number | null;
  imageHeight: number | null;
} & (
  | {
      status: 'completed';
      plainTextPreview: string;
      /** Text around a search match, with markers for `snippetSegments`. */
      matchSnippet: string | null;
      /** Kinds of content detected when the result was stored. */
      contentTags: import('$lib/generated/bindings').ContentTag[];
      hasPlainText: boolean;
      canRestoreFormatting: boolean;
      hasFirstInferenceOutput: boolean;
      hasSecondInferenceOutput: boolean;
    }
  | { status: 'pending' }
  | {
      status: 'failed';
      errorCode: string;
      errorMessage: string;
      errorDetails: string | null;
      hasRawInferenceOutput: boolean;
    }
);

export type HistoryFormattingState = {
  id: string;
  settings: import('$lib/generated/bindings').ProcessingConversionSettings;
  previewHtml: string;
  /** True until the stored preview of this entry has been read. */
  previewLoading: boolean;
  hasPlainText: boolean;
  changed: boolean;
  loading: boolean;
  error: string | null;
};

export type HistoryViewState = {
  entries: HistoryEntry[];
  selectedId: string | null;
  loaded: boolean;
  loading: boolean;
  loadingMore: boolean;
  /** Whether saved entries older than the loaded ones exist. */
  hasMore: boolean;
  filter: import('$lib/history-search').HistoryViewFilter;
  facets: import('$lib/generated/bindings').HistoryFacets;
  error: string | null;
  debugCopied: {
    id: string;
    kind: import('$lib/generated/bindings').HistoryDebugKind;
  } | null;
  resultCopied: { id: string; format: 'rich' | 'plain' } | null;
  failureCopiedId: string | null;
  formatting: HistoryFormattingState | null;
};
