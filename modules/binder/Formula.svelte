<script lang="ts">
  import temml from 'temml';

  let { tex }: { tex: string } = $props();

  function formula(element: HTMLElement, value: string) {
    temml.render(value, element, { displayMode: true });
    return {
      update: (next: string) => temml.render(next, element, { displayMode: true }),
    };
  }
</script>

<div class="formula" use:formula={tex}></div>

<style>
  .formula {
    margin: 6px 0 10px;
    overflow: auto hidden;
    text-align: center;
    font-size: 19px;
  }

  .formula :global(math) {
    font-family: 'Cambria Math', Cambria, serif;
  }
</style>
