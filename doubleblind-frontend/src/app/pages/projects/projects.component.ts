import { Component } from '@angular/core';
import { CommonModule } from '@angular/common';
import {ProjectService} from "../../core/data/project.service";
import {IconTrashComponent} from "../../core/icons/icon-trash/icon-trash.component";
import {ButtonComponent, TextFieldComponent, DropdownComponent} from "@feel/form";
import {CardComponent} from "../../core/components/card/card.component";
import {IconEyeComponent} from "../../core/icons/icon-eye/icon-eye.component";
import {RepositoryService} from "../../core/data/repository.service";

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [CommonModule, IconTrashComponent, ButtonComponent, CardComponent, IconEyeComponent, TextFieldComponent, TextFieldComponent, ButtonComponent, DropdownComponent],
  templateUrl: './projects.component.html',
  styleUrl: './projects.component.scss'
})
export class ProjectsComponent {
  protected readonly projects = this.projectService.getProjects();
  protected readonly repositories= this.repositoryService.getRepositories();
  constructor(
    private readonly projectService:ProjectService,
    private readonly repositoryService:RepositoryService
  ) {
  }

  protected visit_website(subdomain: string) {
      location.href=`https://${subdomain}.science.tanneberger.me`;
  }
}
