export function createState(projection) {
  return {
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedSpecification: null,
    specificationCandidates: null,
    specificationQuery: '',
    specificationKind: 'all',
    specificationEditor: null,
    specificationPreview: null,
    specificationError: null,
    selectedSlice: projection.work.plan?.slices?.[0]?.id || null,
    verificationReceipt: null,
    error: null,
    selectedDiagnosticPhase: 'all',
    selectedScopeMode: 'plan',
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
