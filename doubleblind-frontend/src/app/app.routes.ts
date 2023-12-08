import { Routes } from '@angular/router';

export const routes: Routes = [
  {path: "", loadComponent: ( ) => import ('./pages/landingpage/landingpage.component').then( c => c.LandingpageComponent)},
  {path: "projects", loadComponent: ( ) => import ('./pages/projects/projects.component').then( c => c.ProjectsComponent)}
];
