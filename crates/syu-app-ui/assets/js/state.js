export function createState(projection) {
  return {
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedSpecification: null,
    specificationCandidates: null,
    specificationQuery: '',
    specificationKind: 'all',
    specificationDetailTab: 'information',
    specificationTrace: null,
    specificationTraceRoot: null,
    specificationTraceMode: 'readable',
    specificationTraceNode: null,
    specificationTraceDepth: 1,
    specificationTraceLoading: false,
    specificationEditor: null,
    specificationPreview: null,
    specificationError: null,
    specificationSourceTarget: null,
    specificationSource: null,
    specificationSourceFull: false,
    targetSuggestions: null,
    targetSuggestionSelection: [],
    journeySpecificationExpanded: false,
    journeySpecificationAnchor: null,
    journeyContextTab: 'specification',
    journeyContextItemId: null,
    journeyContextTarget: null,
    journeyContextHistory: [],
    journeyScopeFocus: null,
    journeyDiffRequested: false,
    workTitleEditing: false,
    relatedKind: 'specification',
    selectedSlice: projection.work.plan?.slices?.[0]?.id || null,
    verificationReceipt: null,
    busy: false,
    busyLabel: '',
    error: null,
    selectedDiagnosticPhase: 'all',
    selectedDiagnosticSeverity: 'all',
    diagnosticSort: 'severity',
    diagnosticContext: projection.diagnostics?.validation?.context || 'workspace',
    diagnosticRange: '',
    readinessFilter: 'all',
    readinessSort: 'attention',
    selectedScopeMode: 'plan',
    selectedScopeTab: 'change',
    scopeRange: '',
    scopeLoading: false,
    scopeError: null,
    scopeDiff: null,
    scopeDiffLoading: false,
    scopeDiffError: null,
  };
}

export function replaceProjection(state, projection) {
  const planSlices = projection.work.plan?.slices || [];
  const selectedSlice = projection.work.selected_slice
    || (planSlices.some(slice => slice.id === state.selectedSlice) ? state.selectedSlice : planSlices[0]?.id || null);
  return {
    ...state,
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedSlice,
  };
}

export function selectSlice(state, sliceId) {
  return { ...state, selectedSlice: sliceId };
}
