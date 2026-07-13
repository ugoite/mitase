export function createState(projection) {
  return {
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedSpecification: null,
    selectedSlice: projection.work.plan?.slices?.[0]?.id || null,
    selectedDiagnosticPhase: 'all',
    selectedScopeMode: 'plan',
  };
}

export function replaceProjection(state, projection) {
  return {
    ...state,
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedSlice: state.selectedSlice || projection.work.plan?.slices?.[0]?.id || null,
  };
}

export function selectSlice(state, sliceId) {
  return { ...state, selectedSlice: sliceId };
}
