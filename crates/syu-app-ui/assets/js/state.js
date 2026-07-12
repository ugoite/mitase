export function createState(projection) {
  return {
    projection,
    selectedPage: projection.navigation.selected_page,
    selectedItem: null,
    selectedSlice: projection.work.plan?.slices?.[0]?.id || null,
    draft: null,
  };
}

export function replaceProjection(state, projection) {
  return { ...state, projection, selectedPage: projection.navigation.selected_page };
}
