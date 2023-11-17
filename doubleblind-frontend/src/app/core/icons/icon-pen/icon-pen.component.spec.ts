import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IconPenComponent } from './icon-pen.component';

describe('IconPenComponent', () => {
  let component: IconPenComponent;
  let fixture: ComponentFixture<IconPenComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [IconPenComponent]
    });
    fixture = TestBed.createComponent(IconPenComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
